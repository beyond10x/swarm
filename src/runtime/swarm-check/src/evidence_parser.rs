//! The legacy JSON dialect: ordinary JSON plus NaN/Infinity/-Infinity and lone escaped
//! surrogates. Syntax errors are still errors. Containers use a heap stack rather than recursion.
use crate::{evidence::Value, text::Text};

pub(crate) fn parse(source: &str) -> Result<Value, String> {
    let mut parser = Parser { source, at: 0 };
    let value = parser.value()?;
    parser.space();
    if parser.at != source.len() {
        return Err(parser.error("trailing data"));
    }
    Ok(value)
}
struct Parser<'a> {
    source: &'a str,
    at: usize,
}
impl Parser<'_> {
    fn error(&self, what: &str) -> String {
        format!("{what} at byte {}", self.at)
    }
    fn byte(&self) -> Option<u8> {
        self.source.as_bytes().get(self.at).copied()
    }
    fn space(&mut self) {
        while self
            .byte()
            .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
        {
            self.at += 1;
        }
    }
    fn consume(&mut self, token: &str) -> bool {
        if self.source[self.at..].starts_with(token) {
            self.at += token.len();
            true
        } else {
            false
        }
    }
    fn punctuation(&mut self, byte: u8) -> Result<(), String> {
        self.space();
        if self.byte() == Some(byte) {
            self.at += 1;
            Ok(())
        } else {
            Err(self.error("unexpected token"))
        }
    }
    fn value(&mut self) -> Result<Value, String> {
        enum Frame {
            Array(Vec<Value>),
            Object(Vec<(Text, Value)>, Text),
        }
        let mut stack = vec![];
        loop {
            self.space();
            let mut value = match self.byte() {
                Some(b'[' | b'{') => {
                    let object = self.byte() == Some(b'{');
                    self.at += 1;
                    self.space();
                    if self.byte() == Some(if object { b'}' } else { b']' }) {
                        self.at += 1;
                        if object {
                            Value::Object(vec![])
                        } else {
                            Value::Array(vec![])
                        }
                    } else {
                        stack.push(if object {
                            let key = self.string()?;
                            self.punctuation(b':')?;
                            Frame::Object(vec![], key)
                        } else {
                            Frame::Array(vec![])
                        });
                        continue;
                    }
                }
                Some(b'"') => Value::String(self.string()?),
                Some(b't') if self.consume("true") => Value::Bool(true),
                Some(b'f') if self.consume("false") => Value::Bool(false),
                Some(b'n') if self.consume("null") => Value::Null,
                Some(b'N') if self.consume("NaN") => Value::Float(f64::NAN),
                Some(b'I') if self.consume("Infinity") => Value::Float(f64::INFINITY),
                Some(b'-') if self.consume("-Infinity") => Value::Float(f64::NEG_INFINITY),
                Some(b'-' | b'0'..=b'9') => self.number()?,
                _ => return Err(self.error("expected a JSON value")),
            };
            loop {
                let Some(frame) = stack.last_mut() else {
                    return Ok(value);
                };
                let closing = match frame {
                    Frame::Array(items) => {
                        items.push(value);
                        b']'
                    }
                    Frame::Object(items, key) => {
                        if let Some((_, old)) = items.iter_mut().find(|(k, _)| k == key) {
                            *old = value;
                        } else {
                            items.push((std::mem::take(key), value));
                        }
                        b'}'
                    }
                };
                self.space();
                if self.byte() == Some(closing) {
                    self.at += 1;
                    value = match stack.pop().unwrap() {
                        Frame::Array(items) => Value::Array(items),
                        Frame::Object(items, _) => Value::Object(items),
                    };
                } else {
                    self.punctuation(b',')?;
                    if let Frame::Object(_, key) = stack.last_mut().unwrap() {
                        self.space();
                        *key = self.string()?;
                        self.punctuation(b':')?;
                    }
                    break;
                }
            }
        }
    }
    fn hex(&mut self) -> Result<u32, String> {
        let end = self
            .at
            .checked_add(4)
            .ok_or_else(|| self.error("incomplete Unicode escape"))?;
        let bytes = self
            .source
            .as_bytes()
            .get(self.at..end)
            .ok_or_else(|| self.error("incomplete Unicode escape"))?;
        let mut value = 0;
        for b in bytes {
            value = value * 16
                + (*b as char)
                    .to_digit(16)
                    .ok_or_else(|| self.error("invalid Unicode escape"))?;
        }
        self.at = end;
        Ok(value)
    }
    fn string(&mut self) -> Result<Text, String> {
        self.punctuation(b'"')?;
        let mut out = Text::default();
        loop {
            match self.byte() {
                None => return Err(self.error("unterminated string")),
                Some(b'"') => {
                    self.at += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.at += 1;
                    let escaped = self.byte().ok_or_else(|| self.error("incomplete escape"))?;
                    self.at += 1;
                    let point = match escaped {
                        b'"' => 0x22,
                        b'\\' => 0x5c,
                        b'/' => 0x2f,
                        b'b' => 8,
                        b'f' => 12,
                        b'n' => 10,
                        b'r' => 13,
                        b't' => 9,
                        b'u' => {
                            let first = self.hex()?;
                            if (0xd800..=0xdbff).contains(&first)
                                && self.source[self.at..].starts_with("\\u")
                            {
                                let after_first = self.at;
                                self.at += 2;
                                match self.hex() {
                                    Ok(second) if (0xdc00..=0xdfff).contains(&second) => {
                                        0x10000 + ((first - 0xd800) << 10) + (second - 0xdc00)
                                    }
                                    _ => {
                                        self.at = after_first;
                                        first
                                    }
                                }
                            } else {
                                first
                            }
                        }
                        _ => return Err(self.error("invalid escape")),
                    };
                    out.push(point);
                }
                Some(0..=0x1f) => return Err(self.error("raw control character in string")),
                Some(_) => {
                    let c = self.source[self.at..].chars().next().unwrap();
                    self.at += c.len_utf8();
                    out.push(c as u32);
                }
            }
        }
    }
    fn number(&mut self) -> Result<Value, String> {
        let start = self.at;
        if self.byte() == Some(b'-') {
            self.at += 1;
        }
        match self.byte() {
            Some(b'0') => self.at += 1,
            Some(b'1'..=b'9') => {
                self.at += 1;
                while self.byte().is_some_and(|b| b.is_ascii_digit()) {
                    self.at += 1;
                }
            }
            _ => return Err(self.error("invalid number")),
        }
        let mut float = false;
        if self.byte() == Some(b'.') {
            float = true;
            self.at += 1;
            self.digits()?;
        }
        if self.byte().is_some_and(|b| matches!(b, b'e' | b'E')) {
            float = true;
            self.at += 1;
            if self.byte().is_some_and(|b| matches!(b, b'+' | b'-')) {
                self.at += 1;
            }
            self.digits()?;
        }
        let raw = &self.source[start..self.at];
        if float {
            raw.parse()
                .map(Value::Float)
                .map_err(|_| self.error("invalid float"))
        } else {
            raw.parse()
                .map(Value::Integer)
                .map_err(|_| self.error("invalid integer"))
        }
    }
    fn digits(&mut self) -> Result<(), String> {
        let start = self.at;
        while self.byte().is_some_and(|b| b.is_ascii_digit()) {
            self.at += 1;
        }
        if start == self.at {
            Err(self.error("expected digits"))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn malformed_deep_input_rejects_without_call_stack_growth() {
        for (open, close) in [("[", "]"), ("{\"k\":", "}")] {
            let unfinished = format!("{}null{}", open.repeat(100_000), close.repeat(99_999));
            assert!(super::parse(&unfinished).is_err());
        }
    }
}
