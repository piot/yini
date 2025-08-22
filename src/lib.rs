/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/piot/yini
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use seq_map::SeqMap;

#[derive(Debug, Clone)]
pub enum ErrorKind {
    ExpectedValueOnSameLine,
    ExpectedNewlineAfterKeyValue,
    ExpectedColonAfterKey,
    UnterminatedBlock,
    UnterminatedString,
    ExpectedCommaBetweenArrayItems,
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
    Tuple(Vec<Value>),
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
    #[must_use]
    pub const fn new(input: &'a str) -> Self {
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
            self.skip_horizontal_ws();

            self.skip_horizontal_ws();
            if self.peek_byte() == Some(b'{') {
                // accept shorthand without colon
            } else if !self.consume_colon() {
                self.skip_to_next_line();
                continue;
            }

            self.skip_horizontal_ws();

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

            let val = self.parse_field_value();

            let _ = root.insert(key, val);
            self.require_newline_or_eof();
            self.skip_ws_and_comments();
        }
        root
    }

    #[must_use]
    pub fn errors(&self) -> &[ParseError] {
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
            self.skip_horizontal_ws();

            self.skip_horizontal_ws();
            if self.peek_byte() == Some(b'{') {
                // accept shorthand without colon
            } else if !self.consume_colon() {
                self.skip_to_next_line();
                continue;
            }

            self.skip_horizontal_ws();

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

            let val = self.parse_field_value();
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
            self.skip_ws_and_comments();

            // End of array
            if self.peek_byte() == Some(b']') {
                self.next_byte();
                return array;
            }

            // Parse first value for this element
            let first = self.parse_value();
            self.skip_ws_and_comments();

            if self.is_eof() {
                // End of input after first value
                array.push(first);
                return array;
            }

            match self.peek_byte() {
                // Single-value element immediately followed by a delimiter
                Some(b',') => {
                    array.push(first);
                    self.next_byte();
                }
                Some(b']') => {
                    array.push(first);
                    self.next_byte();
                    return array;
                }
                _ => {
                    // Parse tuple elements (2 or more values before comma/bracket)
                    let mut tuple_items = vec![first];

                    loop {
                        let prev_pos = self.pos;
                        if self.is_eof() {
                            // End of input in tuple
                            array.push(Value::Tuple(tuple_items));
                            return array;
                        }
                        let next_value = self.parse_value();
                        tuple_items.push(next_value);
                        self.skip_ws_and_comments();

                        // If no progress, break to avoid infinite loop
                        if self.pos == prev_pos {
                            self.errors.push(ParseError {
                                line: self.line,
                                column: self.column,
                                kind: ErrorKind::UnexpectedEndOfInput,
                            });
                            array.push(Value::Tuple(tuple_items));
                            return array;
                        }

                        match self.peek_byte() {
                            Some(b',') => {
                                array.push(Value::Tuple(tuple_items));
                                self.next_byte();
                                break;
                            }
                            Some(b']') => {
                                array.push(Value::Tuple(tuple_items));
                                self.next_byte();
                                return array;
                            }
                            Some(_) => {}
                            None => {
                                self.errors.push(ParseError {
                                    line: self.line,
                                    column: self.column,
                                    kind: ErrorKind::UnexpectedEndOfInput,
                                });
                                array.push(Value::Tuple(tuple_items));
                                return array;
                            }
                        }
                    }
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
            Some(b'(') => {
                // parenthesized tuple
                self.parse_tuple()
            }
            Some(b'"') => {
                let s = self.parse_string();
                Value::Str(s)
            }
            Some(b'{') => {
                self.next_byte();
                Value::Object(self.parse_object())
            }
            Some(b'[') => {
                self.next_byte();
                Value::Array(self.parse_array())
            }
            Some(b'-' | b'0'..=b'9') => self.parse_numeric(),
            Some(_) => {
                let id = self.parse_identifier_or_string();
                if id == "true" {
                    Value::Bool(true)
                } else if id == "false" {
                    Value::Bool(false)
                } else {
                    Value::Str(id)
                }
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

    fn parse_field_value(&mut self) -> Value {
        self.skip_horizontal_ws();

        // If parenthesized tuple, parse it
        if self.peek_byte() == Some(b'(') {
            return self.parse_tuple();
        }

        let start_pos = self.pos;
        // parse first token/value
        let first = self.parse_value();
        self.skip_horizontal_ws();

        match self.peek_byte() {
            Some(b'\n' | b'#' | b'}') | None => {
                // single value
                first
            }
            Some(_) => {
                // Move to line end or comment
                while let Some(b) = self.peek_byte() {
                    if b == b'\n' || b == b'#' {
                        break;
                    }
                    self.next_byte();
                }
                // slice from start_pos..pos (includes the first token and whitespace) and trim
                let s = String::from_utf8_lossy(&self.input[start_pos..self.pos])
                    .trim()
                    .to_string();
                if s.is_empty() {
                    // fallback
                    self.pos = start_pos;
                    first
                } else {
                    Value::Str(s)
                }
            }
        }
    }

    fn parse_tuple(&mut self) -> Value {
        // Assumes current peek is '('
        self.next_byte(); // consume '('
        let mut items = Vec::new();

        loop {
            self.skip_ws_and_comments();

            if self.peek_byte() == Some(b')') {
                self.next_byte();
                break;
            }

            if self.is_eof() {
                self.errors.push(ParseError {
                    line: self.line,
                    column: self.column,
                    kind: ErrorKind::UnexpectedEndOfInput,
                });
                break;
            }

            let v = match self.peek_byte() {
                Some(b'"' | b'{' | b'[' | b'(' | b'-' | b'0'..=b'9') => self.parse_value(),
                Some(_) => {
                    // collect until comma, ')' or end-of-input/comment/newline
                    let start = self.pos;
                    while let Some(b) = self.peek_byte() {
                        if b == b',' || b == b')' || b == b'#' || b == b'\n' {
                            break;
                        }
                        self.next_byte();
                    }
                    let s = String::from_utf8_lossy(&self.input[start..self.pos])
                        .trim()
                        .to_string();
                    if s.is_empty() {
                        // fallback to parse_value to generate an error or value
                        self.parse_value()
                    } else if s == "true" {
                        Value::Bool(true)
                    } else if s == "false" {
                        Value::Bool(false)
                    } else {
                        Value::Str(s)
                    }
                }
                None => {
                    self.errors.push(ParseError {
                        line: self.line,
                        column: self.column,
                        kind: ErrorKind::UnexpectedEndOfInput,
                    });
                    break;
                }
            };
            items.push(v);

            self.skip_ws_and_comments();

            match self.peek_byte() {
                Some(b',') => {
                    self.next_byte();
                }
                Some(b')') => {
                    self.next_byte();
                    break;
                }
                Some(b'#' | b'\n') => {
                    // allow comments/newlines inside tuple, keep looping
                }
                Some(_) => {
                    // unexpected token, try to recover by consuming until comma or ')'
                    self.errors.push(ParseError {
                        line: self.line,
                        column: self.column,
                        kind: ErrorKind::ExpectedCommaBetweenArrayItems,
                    });
                    // consume until ',' or ')' or eof
                    loop {
                        match self.peek_byte() {
                            Some(b',' | b')') => {
                                self.next_byte();
                                break;
                            }
                            Some(_) => {
                                let _ = self.next_byte();
                            }
                            None => break,
                        }
                    }
                }
                None => break,
            }
        }

        Value::Tuple(items)
    }

    fn parse_identifier_or_string(&mut self) -> String {
        self.skip_ws_and_comments();
        if self.peek_byte() == Some(b'"') {
            self.parse_string()
        } else {
            let start = self.pos;
            while let Some(&b) = self.input.get(self.pos) {
                let c = b as char;
                if c.is_whitespace()
                    || c == '{'
                    || c == '}'
                    || c == '['
                    || c == ']'
                    || c == ','
                    || c == ':'
                {
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
            if let Ok(n) = s.parse::<f64>() {
                Value::Num(n)
            } else {
                self.errors.push(ParseError {
                    line: self.line,
                    column: self.column,
                    kind: ErrorKind::InvalidFloatFormat(s.to_string()),
                });
                Value::Num(0.0)
            }
        } else if let Ok(n) = s.parse::<i64>() {
            Value::Int(n)
        } else {
            self.errors.push(ParseError {
                line: self.line,
                column: self.column,
                kind: ErrorKind::InvalidIntegerFormat(s.to_string()),
            });
            Value::Int(0)
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

    fn consume_colon(&mut self) -> bool {
        if self.peek_byte() == Some(b':') {
            self.next_byte();
            true
        } else {
            self.errors.push(ParseError {
                line: self.line,
                column: self.column,
                kind: ErrorKind::ExpectedColonAfterKey,
            });
            false
        }
    }

    fn skip_to_next_line(&mut self) {
        while let Some(b) = self.peek_byte() {
            if b == b'\n' {
                self.next_byte();
                break;
            }
            self.next_byte();
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

    // advance_bytes removed; not needed anymore

    const fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }
}

impl Value {
    #[must_use]
    pub const fn as_object(&self) -> Option<&Object> {
        if let Self::Object(o) = self {
            Some(o)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        if let Self::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_num(&self) -> Option<f64> {
        if let Self::Num(n) = *self {
            Some(n)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_int(&self) -> Option<i64> {
        if let Self::Int(i) = *self {
            Some(i)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(b) = *self {
            Some(b)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_array(&self) -> Option<&Vec<Self>> {
        if let Self::Array(a) = self {
            Some(a)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_tuple(&self) -> Option<&[Self]> {
        if let Self::Tuple(items) = self {
            Some(items)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_pair(&self) -> Option<(&Self, &Self)> {
        if let Self::Tuple(items) = self {
            if items.len() >= 2 {
                Some((&items[0], &items[1]))
            } else {
                None
            }
        } else {
            None
        }
    }
}
