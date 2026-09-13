//! Compatibility semantics of the original evidence reader, isolated from runtime JSON.
//!
//! Ordinary `serde_json::Value` remains unchanged: its map order participates in persisted
//! runtime request hashes. This local value retains evidence member order and integer precision.
use crate::text::Text;
use num_bigint::BigInt;
use regex::Regex;
use serde::{Deserialize, Deserializer, de};
use serde_json::value::RawValue;
use std::ops::Index;
use std::sync::LazyLock;

#[derive(Debug, Default)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Integer(BigInt),
    Float(f64),
    String(Text),
    Array(Vec<Value>),
    Object(Vec<(Text, Value)>),
}
impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = Box::<RawValue>::deserialize(deserializer)?;
        Self::parse(raw.get()).map_err(de::Error::custom)
    }
}
impl Value {
    pub(crate) fn parse(raw: &str) -> Result<Self, String> {
        crate::evidence_parser::parse(raw)
    }
    pub fn as_str(&self) -> Option<&str> {
        if let Self::String(s) = self {
            Some(s.encoded())
        } else {
            None
        }
    }
    pub fn as_array(&self) -> Option<&Vec<Self>> {
        if let Self::Array(a) = self {
            Some(a)
        } else {
            None
        }
    }
    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
    /// `isinstance(value, int)`: bool is an integer, but an integral float is not.
    pub fn integer(&self) -> Option<BigInt> {
        match self {
            Self::Bool(b) => Some(BigInt::from(*b)),
            Self::Integer(i) => Some(i.clone()),
            _ => None,
        }
    }
    /// Numeric equality is different from integer membership. Integral floats also join turns.
    pub fn equals_turn(&self, turn: i64) -> bool {
        match self {
            Self::Float(f) => *f == turn as f64,
            _ => self.integer().is_some_and(|i| i == BigInt::from(turn)),
        }
    }
    pub fn truthy(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(b) => *b,
            Self::Integer(i) => *i != BigInt::from(0),
            Self::Float(f) => *f != 0.0,
            Self::String(s) => !s.encoded().is_empty(),
            Self::Array(a) => !a.is_empty(),
            Self::Object(o) => !o.is_empty(),
        }
    }
    pub fn python(&self) -> String {
        if let Self::String(s) = self {
            s.encoded().to_owned()
        } else {
            self.repr()
        }
    }
    fn repr(&self) -> String {
        self.render_value(false)
    }
    /// Python's list conversion before `listed`: arrays yield values, objects yield keys,
    /// strings yield characters. Falsy values are replaced by an empty list at the call site.
    pub fn listed_items(&self) -> Vec<String> {
        match self {
            Self::Array(a) => a.iter().map(Self::python).collect(),
            Self::Object(o) => o.iter().map(|(k, _)| k.encoded().to_owned()).collect(),
            Self::String(s) => s.characters(),
            _ => vec![],
        }
    }
    /// The old ceiling reader searched `json.dumps(event.data)`, not a Python repr.
    pub fn json(&self) -> String {
        self.render_value(true)
    }
    fn render_value(&self, json: bool) -> String {
        enum Piece<'a> {
            Value(&'a Value),
            Key(&'a Text),
            Token(&'static str),
        }
        let mut pending = vec![Piece::Value(self)];
        let mut out = String::new();
        while let Some(piece) = pending.pop() {
            let value = match piece {
                Piece::Token(token) => {
                    out.push_str(token);
                    continue;
                }
                Piece::Key(key) => {
                    out.push_str(&if json {
                        json_quote(key)
                    } else {
                        python_quote(key)
                    });
                    continue;
                }
                Piece::Value(value) => value,
            };
            match value {
                Self::Array(items) => {
                    out.push('[');
                    pending.push(Piece::Token("]"));
                    for (index, item) in items.iter().enumerate().rev() {
                        pending.push(Piece::Value(item));
                        if index > 0 {
                            pending.push(Piece::Token(", "));
                        }
                    }
                }
                Self::Object(items) => {
                    out.push('{');
                    pending.push(Piece::Token("}"));
                    for (index, (key, value)) in items.iter().enumerate().rev() {
                        pending.push(Piece::Value(value));
                        pending.push(Piece::Token(": "));
                        pending.push(Piece::Key(key));
                        if index > 0 {
                            pending.push(Piece::Token(", "));
                        }
                    }
                }
                Self::Null => out.push_str(if json { "null" } else { "None" }),
                Self::Bool(true) => out.push_str(if json { "true" } else { "True" }),
                Self::Bool(false) => out.push_str(if json { "false" } else { "False" }),
                Self::Integer(i) => out.push_str(&i.to_string()),
                Self::Float(f) if json && f.is_infinite() => {
                    out.push_str(if f.is_sign_negative() {
                        "-Infinity"
                    } else {
                        "Infinity"
                    })
                }
                Self::Float(f) if json && f.is_nan() => out.push_str("NaN"),
                Self::Float(f) => out.push_str(&python_float(*f)),
                Self::String(s) => {
                    out.push_str(&if json { json_quote(s) } else { python_quote(s) })
                }
            }
        }
        out
    }
}
// Evidence may have tens of thousands of nested containers. Parsing, rendering, cloning and
// destruction all avoid the call stack. The canonical local JSON preserves every value variant.
impl Clone for Value {
    fn clone(&self) -> Self {
        Self::parse(&self.json()).expect("locally rendered evidence parses")
    }
}
impl Drop for Value {
    fn drop(&mut self) {
        fn children(value: &mut Value, pending: &mut Vec<Value>) {
            match value {
                Value::Array(items) => pending.append(items),
                Value::Object(items) => {
                    pending.extend(std::mem::take(items).into_iter().map(|(_, v)| v))
                }
                _ => {}
            }
        }
        let mut pending = vec![];
        children(self, &mut pending);
        while let Some(mut child) = pending.pop() {
            children(&mut child, &mut pending);
        }
    }
}
impl Index<&str> for Value {
    type Output = Self;
    fn index(&self, key: &str) -> &Self {
        static NULL: Value = Value::Null;
        if let Self::Object(o) = self {
            o.iter()
                .find(|(k, _)| k.encoded() == key)
                .map(|(_, v)| v)
                .unwrap_or(&NULL)
        } else {
            &NULL
        }
    }
}
impl PartialEq<&str> for Value {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == Some(*other)
    }
}
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Array(a), Self::Array(b)) => a == b,
            (Self::Object(a), Self::Object(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .all(|(k, v)| b.iter().any(|(bk, bv)| k == bk && v == bv))
            }
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::Float(f), b) | (b, Self::Float(f)) => float_integer(*f)
                .zip(b.integer())
                .is_some_and(|(a, b)| a == b),
            (a, b) => a.integer().zip(b.integer()).is_some_and(|(a, b)| a == b),
        }
    }
}
fn float_integer(f: f64) -> Option<BigInt> {
    if !f.is_finite() || f.fract() != 0.0 {
        return None;
    }
    if f == 0.0 {
        return Some(BigInt::from(0));
    }
    let bits = f.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32 - 1023 - 52;
    let mut value = BigInt::from((bits & ((1u64 << 52) - 1)) | (1u64 << 52));
    if exponent >= 0 {
        value <<= exponent as usize;
    } else {
        value >>= (-exponent) as usize;
    }
    Some(if f.is_sign_negative() { -value } else { value })
}
fn python_float(f: f64) -> String {
    if f.is_nan() {
        return "nan".into();
    }
    let text = format!("{f:?}");
    if let Some((mantissa, exponent)) = text.split_once('e') {
        let exponent: i32 = exponent.parse().expect("Rust formats a numeric exponent");
        format!("{mantissa}e{exponent:+03}")
    } else {
        text
    }
}
static NONPRINTABLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\p{C}\p{Z}]").unwrap());
fn python_quote(s: &Text) -> String {
    let points: Vec<_> = s.points().collect();
    let quote = if points.contains(&0x27) && !points.contains(&0x22) {
        '"'
    } else {
        '\''
    };
    let mut out = String::from(quote);
    for point in points {
        match point {
            0x5c => out.push_str("\\\\"),
            10 => out.push_str("\\n"),
            13 => out.push_str("\\r"),
            9 => out.push_str("\\t"),
            point if point == quote as u32 => {
                out.push('\\');
                out.push(quote);
            }
            0x20 => out.push(' '),
            point => {
                let printable = char::from_u32(point)
                    .is_some_and(|c| !NONPRINTABLE.is_match(c.encode_utf8(&mut [0; 4])));
                if printable {
                    out.push(char::from_u32(point).unwrap());
                } else if point <= 0xff {
                    out.push_str(&format!("\\x{point:02x}"));
                } else if point <= 0xffff {
                    out.push_str(&format!("\\u{point:04x}"));
                } else {
                    out.push_str(&format!("\\U{point:08x}"));
                }
            }
        }
    }
    out.push(quote);
    out
}
fn json_quote(s: &Text) -> String {
    s.json()
}
