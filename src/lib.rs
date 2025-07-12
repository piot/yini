use seq_map::SeqMap;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum Value {
    Str(String),
    Int(i64),
    Num(f64),
    Bool(bool),
    Object(Object),
}

pub type Object = SeqMap<String, Value>;


pub struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    line: usize,
    column: usize,
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    /// Create a new parser over the input string.
    #[must_use] pub fn new(input: &'a str) -> Self {
        Parser {
            input: input.as_bytes(),
            pos: 0,
            line: 1,
            column: 1,
            errors: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> Object {
        let mut root: Object = SeqMap::new();
        self.skip_ws_and_comments();
        while !self.is_eof() {
            let key = self.parse_key();
            self.skip_ws_and_comments();

            let val = if self.peek_byte() == Some(b'{') {
                self.next_byte(); // consume '{'
                Value::Object(self.parse_object())
            } else {
                self.parse_value()
            };

            let _ = root.insert(key, val);
            self.skip_ws_and_comments();
        }
        root
    }

    #[must_use] pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    fn parse_object(&mut self) -> Object {
        let mut map = SeqMap::new();
        self.skip_ws_and_comments();

        while let Some(b) = self.peek_byte() {
            if b == b'}' {
                self.next_byte();
                return map;
            }
            let key = self.parse_key();
            self.skip_ws_and_comments();

            let val = if self.peek_byte() == Some(b'{') {
                self.next_byte();
                Value::Object(self.parse_object())
            } else {
                self.parse_value()
            };
            let _ = map.insert(key, val);
            self.skip_ws_and_comments();
        }

        self.errors.push(ParseError {
            line: self.line,
            column: self.column,
            message: "Unterminated block, missing '}'".into(),
        });

        map
    }

    fn parse_key(&mut self) -> String {
        self.skip_ws_and_comments();
        self.parse_identifier_or_string()
    }

    fn parse_value(&mut self) -> Value {
        self.skip_ws_and_comments();
        match self.peek_byte() {
            Some(b'"') => {
                let s = self.parse_string();
                Value::Str(s)
            }
            Some(b'-' | b'0'..=b'9') => self.parse_numeric(),
            Some(b't' | b'f') => Value::Bool(self.parse_boolean()),
            Some(_) => {
                let id = self.parse_identifier_or_string();
                Value::Str(id)
            }
            None => {
                self.errors.push(ParseError {
                    line: self.line,
                    column: self.column,
                    message: "Unexpected end of input when expecting value".into(),
                });
                Value::Str(String::new())
            }
        }
    }

    fn parse_identifier_or_string(&mut self) -> String {
        self.skip_ws_and_comments();
        if self.peek_byte() == Some(b'"') {
            self.parse_string()
        } else {
            let start = self.pos;
            while let Some(&b) = self.input.get(self.pos) {
                let c = b as char;
                if c.is_whitespace() || c == '{' || c == '}' {
                    break;
                }
                self.advance_byte(b);
            }
            String::from_utf8_lossy(&self.input[start..self.pos]).into_owned()
        }
    }

    fn parse_string(&mut self) -> String {
        self.next_byte();
        let mut raw = Vec::new();
        while let Some(b) = self.next_byte() {
            match b {
                b'"' => return String::from_utf8_lossy(&raw).into_owned(),
                b'\\' => {
                    if let Some(esc) = self.next_byte() {
                        match esc {
                            b'n' => raw.push(b'\n'),
                            b't' => raw.push(b'\t'),
                            b'r' => raw.push(b'\r'),
                            b'"' => raw.push(b'"'),
                            b'\\' => raw.push(b'\\'),
                            other => raw.push(other),
                        }
                    }
                }
                other => raw.push(other),
            }
        }
        // unterminated string
        self.errors.push(ParseError {
            line: self.line,
            column: self.column,
            message: "Unterminated string literal".into(),
        });
        String::from_utf8_lossy(&raw).into_owned()
    }

    fn parse_numeric(&mut self) -> Value {
        let start = self.pos;
        // optional sign
        if let Some(b'-') = self.peek_byte() {
            self.advance_byte(b'-');
        }
        // digits before decimal
        while let Some(&b) = self.input.get(self.pos) {
            let c = b as char;
            if !c.is_ascii_digit() {
                break;
            }
            self.advance_byte(b);
        }
        let is_float = if self.peek_byte() == Some(b'.') {
            // consume '.' and fraction
            self.advance_byte(b'.');
            while let Some(&b) = self.input.get(self.pos) {
                let c = b as char;
                if !c.is_ascii_digit() {
                    break;
                }
                self.advance_byte(b);
            }
            true
        } else {
            false
        };
        let slice = &self.input[start..self.pos];
        let Ok(s) = std::str::from_utf8(slice) else {
                        self.errors.push(ParseError {
                                 line: self.line,
                                 column: self.column,
                                 message: "Invalid UTF-8 in number literal".into(),
                             });
                         return Value::Int(0);
                    };
        if is_float {
            if let Ok(n) = s.parse::<f64>() { Value::Num(n) } else {
                self.errors.push(ParseError {
                    line: self.line,
                    column: self.column,
                    message: format!("Invalid float format: {s}"),
                });
                Value::Num(0.0)
            }
        } else if let Ok(n) = s.parse::<i64>() { Value::Int(n) } else {
            self.errors.push(ParseError {
                line: self.line,
                column: self.column,
                message: format!("Invalid integer format: {s}"),
            });
            Value::Int(0)
        }
    }

    fn parse_boolean(&mut self) -> bool {
        if self
            .input
            .get(self.pos..)
            .is_some_and(|s| s.starts_with(b"true"))
        {
            self.advance_bytes(4);
            true
        } else if self
            .input
            .get(self.pos..)
            .is_some_and(|s| s.starts_with(b"false"))
        {
            self.advance_bytes(5);
            false
        } else {
            self.errors.push(ParseError {
                line: self.line,
                column: self.column,
                message: "Invalid boolean literal".into(),
            });
            false
        }
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while let Some(&b) = self.input.get(self.pos) {
                if (b as char).is_whitespace() {
                    self.advance_byte(b);
                } else {
                    break;
                }
            }
            if self
                .input
                .get(self.pos..)
                .is_some_and(|s| s.starts_with(b"//"))
            {
                while let Some(b) = self.next_byte() {
                    if b == b'\n' {
                        break;
                    }
                }
                continue;
            }
            if self.peek_byte() == Some(b'#') {
                self.advance_byte(b'#');
                while let Some(b) = self.next_byte() {
                    if b == b'\n' {
                        break;
                    }
                }
                continue;
            }
            break;
        }
    }

    fn peek_byte(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn next_byte(&mut self) -> Option<u8> {
        let b = self.peek_byte();
        if let Some(bb) = b {
            self.advance_byte(bb);
        }
        b
    }

    fn advance_byte(&mut self, b: u8) {
        self.pos += 1;
        if b == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
    }

    fn advance_bytes(&mut self, n: usize) {
        for _ in 0..n {
            if let Some(b) = self.peek_byte() {
                self.advance_byte(b);
            }
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }
}

impl Value {
    #[must_use] pub fn as_object(&self) -> Option<&Object> {
        if let Value::Object(o) = self {
            Some(o)
        } else {
            None
        }
    }

    #[must_use] pub fn as_str(&self) -> Option<&str> {
        if let Value::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }

    #[must_use] pub fn as_num(&self) -> Option<f64> {
        if let Value::Num(n) = *self {
            Some(n)
        } else {
            None
        }
    }

    #[must_use] pub fn as_int(&self) -> Option<i64> {
        if let Value::Int(i) = *self { Some(i) } else { None }
    }

    #[must_use] pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(b) = *self {
            Some(b)
        } else {
            None
        }
    }
}
