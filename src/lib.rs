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
    UnexpectedEndOfInput,
    UnexpectedCharacter(char),
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
    Variant(String, Option<Box<Value>>),
    Struct(Struct),
    Array(Vec<Value>),
    Tuple(Vec<Value>),
}

pub type Struct = SeqMap<String, Value>;

pub struct Parser<'a> {
    input: &'a [u8],
    len: usize,
    pos: usize,
    line: usize,
    column: usize,
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    /// Create a new parser over the input string.
    #[must_use]
    pub const fn new(input: &'a str) -> Self {
        let bytes = input.as_bytes();
        Parser {
            input: bytes,
            len: bytes.len(),
            pos: 0,
            line: 1,
            column: 1,
            errors: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> Struct {
        let mut root: Struct = SeqMap::new();
        self.skip_ws_and_comments();
        while !self.is_eof() {
            let key = self.parse_key();

            // If we got an empty key, we hit an unexpected character
            if key.is_empty() {
                if let Some(b) = self.peek_byte() {
                    let ch = b as char;
                    self.errors.push(ParseError {
                        line: self.line,
                        column: self.column,
                        kind: ErrorKind::UnexpectedCharacter(ch),
                    });
                }
                self.synchronize();
                continue;
            }

            // Colon is optional - but must be *immediately* after key (no whitespace)
            if self.peek_byte() == Some(b':') {
                self.next_byte();
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

    fn parse_struct(&mut self) -> Struct {
        let mut map = SeqMap::new();
        self.skip_ws_and_comments();

        while let Some(b) = self.peek_byte() {
            if b == b'}' {
                self.next_byte();
                return map;
            }
            let key = self.parse_key();

            // If we got an empty key, we hit an unexpected character
            if key.is_empty() {
                if let Some(b) = self.peek_byte() {
                    let ch = b as char;
                    self.errors.push(ParseError {
                        line: self.line,
                        column: self.column,
                        kind: ErrorKind::UnexpectedCharacter(ch),
                    });
                }
                self.synchronize();
                continue;
            }

            // Code is repeated here for performance reasons
            // Colon is optional - but must be *immediately* after key (no whitespace)
            if self.peek_byte() == Some(b':') {
                self.next_byte();
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

            if self.is_eof() {
                self.errors.push(ParseError {
                    line: self.line,
                    column: self.column,
                    kind: ErrorKind::UnexpectedEndOfInput,
                });
                return array;
            }

            // Parse a single value (tuples must be explicitly wrapped in parentheses)
            let value = self.parse_value();
            array.push(value);

            self.skip_ws_and_comments();

            match self.peek_byte() {
                Some(b']') => {
                    // End of array
                    self.next_byte();
                    return array;
                }
                Some(_) => {
                    // Whitespace-separated item, continue to next iteration
                    // No error needed since commas are optional
                }
                None => {
                    self.errors.push(ParseError {
                        line: self.line,
                        column: self.column,
                        kind: ErrorKind::UnexpectedEndOfInput,
                    });
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
                Value::Struct(self.parse_struct())
            }
            Some(b'[') => {
                self.next_byte();
                Value::Array(self.parse_array())
            }
            Some(b':') => {
                // Variant (like :Fullscreen, :north, etc.)
                self.next_byte(); // consume ':'
                let id = self.parse_variant_name();

                // Check for optional payload: (tuple) {object} [array]
                // NO whitespace allowed between variant name and payload
                let payload = match self.peek_byte() {
                    Some(b'(') => {
                        // Tuple payload: :variant(a, b, c)
                        Some(Box::new(self.parse_tuple()))
                    }
                    Some(b'{') => {
                        // Object payload: :variant{key: value}
                        self.next_byte(); // consume '{'
                        Some(Box::new(Value::Struct(self.parse_struct())))
                    }
                    Some(b'[') => {
                        // Array payload: :variant[1, 2, 3]
                        self.next_byte(); // consume '['
                        Some(Box::new(Value::Array(self.parse_array())))
                    }
                    _ => None,
                };

                Value::Variant(id, payload)
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
                let trimmed = self.slice_to_str(start_pos, self.pos).trim();
                if trimmed.is_empty() {
                    // fallback
                    self.pos = start_pos;
                    first
                } else {
                    Value::Str(trimmed.to_owned())
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
                Some(b'"' | b'{' | b'[' | b'(' | b'-' | b'0'..=b'9' | b':') => self.parse_value(),
                Some(_) => {
                    // collect until comma, ')' or end-of-input/comment/newline
                    let start = self.pos;
                    while let Some(b) = self.peek_byte() {
                        if b == b')' || b == b'#' || b == b'\n' {
                            break;
                        }
                        self.next_byte();
                    }
                    let trimmed = self.slice_to_str(start, self.pos).trim();
                    if trimmed.is_empty() {
                        // fallback to parse_value to generate an error or value
                        self.parse_value()
                    } else if trimmed == "true" {
                        Value::Bool(true)
                    } else if trimmed == "false" {
                        Value::Bool(false)
                    } else {
                        Value::Str(trimmed.to_owned())
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
                Some(b')') => {
                    self.next_byte();
                    break;
                }
                Some(b'#' | b'\n') => {
                    // allow comments/newlines inside tuple, keep looping
                }
                Some(_) => {
                    // Whitespace-separated item, continue to next iteration
                }
                None => break,
            }
        }

        Value::Tuple(items)
    }

    #[inline]
    fn parse_identifier_or_string(&mut self) -> String {
        self.skip_ws_and_comments();
        if self.peek_byte() == Some(b'"') {
            self.parse_string()
        } else {
            let start = self.pos;
            while self.pos < self.len {
                // SAFETY: We just checked pos < len
                let b = unsafe { *self.input.get_unchecked(self.pos) };
                // Fast delimiter check
                match b {
                    b' ' | b'\t' | b'\n' | b'\r' | b'{' | b'}' | b'[' | b']' | b':' | b'('
                    | b')' => break,
                    _ => {
                        self.pos += 1;
                        self.column += 1;
                    }
                }
            }
            self.slice_to_str(start, self.pos).to_owned()
        }
    }

    #[inline]
    fn slice_to_str(&self, start: usize, end: usize) -> &str {
        debug_assert!(start <= end && end <= self.len);
        // SAFETY: input originates from a valid UTF-8 source string
        unsafe { std::str::from_utf8_unchecked(&self.input[start..end]) }
    }

    #[inline]
    fn parse_variant_name(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.len {
            // SAFETY: We just checked pos < len
            let b = unsafe { *self.input.get_unchecked(self.pos) };
            match b {
                b' ' | b'\t' | b'\n' | b'\r' | b'{' | b'}' | b'[' | b']' | b')' | b'(' | b':' => {
                    break;
                }
                _ => {
                    self.pos += 1;
                    self.column += 1;
                }
            }
        }
        // SAFETY: start and pos are valid indices
        self.slice_to_str(start, self.pos).to_owned()
    }

    fn parse_string(&mut self) -> String {
        self.next_byte();
        let mut raw = Vec::with_capacity(16);
        while let Some(b) = self.next_byte() {
            match b {
                b'"' => {
                    // SAFETY: raw is built from the original UTF-8 input plus ASCII escapes
                    return unsafe { String::from_utf8_unchecked(raw) };
                }
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
        // SAFETY: partial string still contains only bytes from the original UTF-8 input
        unsafe { String::from_utf8_unchecked(raw) }
    }

    #[inline]
    fn parse_numeric(&mut self) -> Value {
        let start = self.pos;
        // optional sign
        if self.peek_byte() == Some(b'-') {
            self.pos += 1;
            self.column += 1;
        }
        // digits before decimal
        while self.pos < self.len {
            // SAFETY: We just checked pos < len
            let b = unsafe { *self.input.get_unchecked(self.pos) };
            if !b.is_ascii_digit() {
                break;
            }
            self.pos += 1;
            self.column += 1;
        }
        let is_float = if self.peek_byte() == Some(b'.') {
            // consume '.' and fraction
            self.pos += 1;
            self.column += 1;
            while self.pos < self.len {
                // SAFETY: We just checked pos < len
                let b = unsafe { *self.input.get_unchecked(self.pos) };
                if !b.is_ascii_digit() {
                    break;
                }
                self.pos += 1;
                self.column += 1;
            }
            true
        } else {
            false
        };
        // SAFETY: start..pos are valid indices within input
        let slice = unsafe { self.input.get_unchecked(start..self.pos) };
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
            // Fast path: skip whitespace using direct byte comparisons
            while self.pos < self.len {
                // SAFETY: We just checked pos < len
                let b = unsafe { *self.input.get_unchecked(self.pos) };
                match b {
                    b' ' | b'\t' | b'\n' | b'\r' => {
                        self.advance_byte(b);
                    }
                    _ => break,
                }
            }

            // Check for comment
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

    #[inline]
    fn skip_horizontal_ws(&mut self) {
        while self.pos < self.len {
            // SAFETY: We just checked pos < len
            let b = unsafe { *self.input.get_unchecked(self.pos) };
            if b == b' ' || b == b'\t' {
                self.pos += 1;
                self.column += 1;
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

    #[inline(always)]
    fn peek_byte(&self) -> Option<u8> {
        if self.pos < self.len {
            // SAFETY: We just checked that pos < len
            Some(unsafe { *self.input.get_unchecked(self.pos) })
        } else {
            None
        }
    }

    #[inline(always)]
    fn next_byte(&mut self) -> Option<u8> {
        if self.pos < self.len {
            // SAFETY: We just checked that pos < len
            let b = unsafe { *self.input.get_unchecked(self.pos) };
            self.advance_byte(b);
            Some(b)
        } else {
            None
        }
    }

    #[inline(always)]
    const fn advance_byte(&mut self, b: u8) {
        self.pos += 1;
        if b == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
    }

    #[inline(always)]
    const fn is_eof(&self) -> bool {
        self.pos >= self.len
    }

    /// Synchronize after an error
    /// Try to find a good place to resume, currently just advancing to the next newline or EOF.
    fn synchronize(&mut self) {
        while let Some(b) = self.peek_byte() {
            if b == b'\n' {
                self.next_byte();
                break;
            }
            self.next_byte();
        }
        self.skip_ws_and_comments();
    }
}

impl Value {
    #[must_use]
    pub const fn as_struct(&self) -> Option<&Struct> {
        if let Self::Struct(o) = self {
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
    pub fn as_variant(&self) -> Option<&str> {
        if let Self::Variant(s, _) = self {
            Some(s)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_variant_with_payload(&self) -> Option<(&str, Option<&Self>)> {
        if let Self::Variant(s, payload) = self {
            Some((s, payload.as_ref().map(std::convert::AsRef::as_ref)))
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
