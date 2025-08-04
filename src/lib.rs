/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/piot/yini
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use seq_map::SeqMap;

#[derive(Debug, Clone)]
pub enum ErrorKind {
    ExpectedValueOnSameLine,
    ExpectedNewlineAfterKeyValue,
    UnterminatedBlock,
    UnterminatedString,
    InvalidUtf8InNumber,
    InvalidFloatFormat(String),
    InvalidIntegerFormat(String),
    InvalidBooleanLiteral,
    UnexpectedEndOfInput,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub line: usize,
    pub column: usize,
    pub kind: ErrorKind,
}

#[derive(Debug, Clone)]
pub enum Value {
    Str(String),
    Int(i64),
    Num(f64),
    Bool(bool),
    Object(Object),
    Array(Vec<Value>),
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
    #[must_use] pub const fn new(input: &'a str) -> Self {
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
            self.skip_horizontal_ws(); // Only skip spaces/tabs, not newlines

            // Check if we have a value on the same line
            if self.peek_byte() == Some(b'\n') || self.is_eof() {
                self.errors.push(ParseError {
                    line: self.line,
                    column: self.column,
                    kind: ErrorKind::ExpectedValueOnSameLine,
                });
                // Skip to next line to continue parsing
                if self.peek_byte() == Some(b'\n') {
                    self.next_byte();
                }
                continue;
            }

            let val = self.parse_value();

            let _ = root.insert(key, val);
            self.require_newline_or_eof();
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
            self.skip_horizontal_ws(); // Only skip spaces/tabs, not newlines

            // Check if we have a value on the same line
            if self.peek_byte() == Some(b'\n') || self.is_eof() {
                self.errors.push(ParseError {
                    line: self.line,
                    column: self.column,
                    kind: ErrorKind::ExpectedValueOnSameLine,
                });
                // Skip to next line to continue parsing
                if self.peek_byte() == Some(b'\n') {
                    self.next_byte();
                }
                continue;
            }

            let val = self.parse_value();
            let _ = map.insert(key, val);
            self.require_newline_or_eof();
            self.skip_ws_and_comments();
        }

        self.errors.push(ParseError {
            line: self.line,
            column: self.column,
            kind: ErrorKind::UnterminatedBlock,
        });

        map
    }

    fn parse_array(&mut self) -> Vec<Value> {
        let mut array = Vec::new();
        self.skip_ws_and_comments();

        // Handle empty array
        if self.peek_byte() == Some(b']') {
            self.next_byte();
            return array;
        }

        loop {
            // Parse value
            let val = self.parse_value();

            array.push(val);
            self.skip_ws_and_comments();

            // Check for end of array
            if self.peek_byte() == Some(b']') {
                self.next_byte();
                return array;
            }

            // Check for comma separator
            if self.peek_byte() == Some(b',') {
                self.next_byte();
                self.skip_ws_and_comments();
                // Allow trailing comma
                if self.peek_byte() == Some(b']') {
                    self.next_byte();
                    return array;
                }
            } else {
                // No comma, check if we're at the end
                if self.peek_byte() == Some(b']') {
                    self.next_byte();
                    return array;
                }
                // Continue parsing without comma (space-separated)
                self.skip_ws_and_comments();
                if self.peek_byte() == Some(b']') {
                    self.next_byte();
                    return array;
                }
            }
        }
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
            Some(b'{') => {
                self.next_byte(); // consume '{'
                Value::Object(self.parse_object())
            }
            Some(b'[') => {
                self.next_byte(); // consume '['
                Value::Array(self.parse_array())
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
                    kind: ErrorKind::UnexpectedEndOfInput,
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
                if c.is_whitespace() || c == '{' || c == '}' || c == '[' || c == ']' || c == ',' {
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
            kind: ErrorKind::UnterminatedString,
        });
        String::from_utf8_lossy(&raw).into_owned()
    }

    fn parse_numeric(&mut self) -> Value {
        let start = self.pos;
        // optional sign
        if self.peek_byte() == Some(b'-') {
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
                                 kind: ErrorKind::InvalidUtf8InNumber,
                             });
                         return Value::Int(0);
                    };
        if is_float {
            if let Ok(n) = s.parse::<f64>() { Value::Num(n) } else {
                self.errors.push(ParseError {
                    line: self.line,
                    column: self.column,
                    kind: ErrorKind::InvalidFloatFormat(s.to_string()),
                });
                Value::Num(0.0)
            }
        } else if let Ok(n) = s.parse::<i64>() { Value::Int(n) } else {
            self.errors.push(ParseError {
                line: self.line,
                column: self.column,
                kind: ErrorKind::InvalidIntegerFormat(s.to_string()),
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
                kind: ErrorKind::InvalidBooleanLiteral,
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

    fn skip_horizontal_ws(&mut self) {
        while let Some(&b) = self.input.get(self.pos) {
            if b == b' ' || b == b'\t' {
                self.advance_byte(b);
            } else {
                break;
            }
        }
    }

    fn require_newline_or_eof(&mut self) {
        self.skip_horizontal_ws();

        if self.is_eof() {
            return;
        }

        if self.peek_byte() == Some(b'\n') {
            return;
        }

        if self.peek_byte() == Some(b'#') {
            return;
        }

        self.errors.push(ParseError {
            line: self.line,
            column: self.column,
            kind: ErrorKind::ExpectedNewlineAfterKeyValue,
        });
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

    const fn advance_byte(&mut self, b: u8) {
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

    const fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }
}

impl Value {
    #[must_use] pub const fn as_object(&self) -> Option<&Object> {
        if let Self::Object(o) = self {
            Some(o)
        } else {
            None
        }
    }

    #[must_use] pub fn as_str(&self) -> Option<&str> {
        if let Self::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }

    #[must_use] pub const fn as_num(&self) -> Option<f64> {
        if let Self::Num(n) = *self {
            Some(n)
        } else {
            None
        }
    }

    #[must_use] pub const fn as_int(&self) -> Option<i64> {
        if let Self::Int(i) = *self { Some(i) } else { None }
    }

    #[must_use] pub const fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(b) = *self {
            Some(b)
        } else {
            None
        }
    }

    #[must_use] pub const fn as_array(&self) -> Option<&Vec<Self>> {
        if let Self::Array(a) = self {
            Some(a)
        } else {
            None
        }
    }
}
