//! Compatibility semantics of the original evidence reader, isolated from runtime JSON.
//!
//! Ordinary `serde_json::Value` remains unchanged: its map order participates in persisted
//! runtime request hashes. This local value retains evidence member order and integer precision.
use num_bigint::BigInt;
use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, Visitor},
};
use serde_json::value::RawValue;
use std::{fmt, ops::Index};

#[derive(Clone, Debug, Default)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Integer(BigInt),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}
impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = Box::<RawValue>::deserialize(deserializer)?;
        Self::parse(raw.get()).map_err(de::Error::custom)
    }
}
struct ObjectVisitor;
impl<'de> Visitor<'de> for ObjectVisitor {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an evidence object")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Value, M::Error> {
        let mut entries: Vec<(String, Value)> = vec![];
        while let Some((key, value)) = map.next_entry::<String, Value>()? {
            // Python keeps a duplicate key's original position and replaces its value.
            if let Some((_, old)) = entries.iter_mut().find(|(k, _)| *k == key) {
                *old = value;
            } else {
                entries.push((key, value));
            }
        }
        Ok(Value::Object(entries))
    }
}
impl Value {
    fn parse(raw: &str) -> Result<Self, String> {
        let raw = raw.trim();
        match raw.as_bytes().first() {
            Some(b'{') => {
                let mut deserializer = serde_json::Deserializer::from_str(raw);
                serde::Deserializer::deserialize_map(&mut deserializer, ObjectVisitor)
                    .map_err(|e| e.to_string())
            }
            Some(b'[') => serde_json::from_str(raw)
                .map(Self::Array)
                .map_err(|e| e.to_string()),
            Some(b'"') => serde_json::from_str(raw)
                .map(Self::String)
                .map_err(|e| e.to_string()),
            Some(b't') => Ok(Self::Bool(true)),
            Some(b'f') => Ok(Self::Bool(false)),
            Some(b'n') => Ok(Self::Null),
            _ if raw.contains(['.', 'e', 'E']) => raw
                .parse()
                .map(Self::Float)
                .map_err(|e: std::num::ParseFloatError| e.to_string()),
            _ => raw
                .parse()
                .map(Self::Integer)
                .map_err(|e: num_bigint::ParseBigIntError| e.to_string()),
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        if let Self::String(s) = self {
            Some(s)
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
            Self::String(s) => !s.is_empty(),
            Self::Array(a) => !a.is_empty(),
            Self::Object(o) => !o.is_empty(),
        }
    }
    pub fn python(&self) -> String {
        if let Self::String(s) = self {
            s.clone()
        } else {
            self.repr()
        }
    }
    fn repr(&self) -> String {
        match self {
            Self::Null => "None".into(),
            Self::Bool(true) => "True".into(),
            Self::Bool(false) => "False".into(),
            Self::Integer(i) => i.to_string(),
            Self::Float(f) => python_float(*f),
            Self::String(s) => python_quote(s),
            Self::Array(a) => format!(
                "[{}]",
                a.iter().map(Self::repr).collect::<Vec<_>>().join(", ")
            ),
            Self::Object(o) => format!(
                "{{{}}}",
                o.iter()
                    .map(|(k, v)| format!("{}: {}", python_quote(k), v.repr()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
    /// Python's list conversion before `listed`: arrays yield values, objects yield keys,
    /// strings yield characters. Falsy values are replaced by an empty list at the call site.
    pub fn listed_items(&self) -> Vec<String> {
        match self {
            Self::Array(a) => a.iter().map(Self::python).collect(),
            Self::Object(o) => o.iter().map(|(k, _)| k.clone()).collect(),
            Self::String(s) => s.chars().map(|c| c.to_string()).collect(),
            _ => vec![],
        }
    }
    /// The old ceiling reader searched `json.dumps(event.data)`, not a Python repr.
    pub fn json(&self) -> String {
        match self {
            Self::Null => "null".into(),
            Self::Bool(b) => b.to_string(),
            Self::Integer(i) => i.to_string(),
            Self::Float(f) if f.is_infinite() => {
                if f.is_sign_negative() {
                    "-Infinity".into()
                } else {
                    "Infinity".into()
                }
            }
            Self::Float(f) => python_float(*f),
            Self::String(s) => json_quote(s),
            Self::Array(a) => format!(
                "[{}]",
                a.iter().map(Self::json).collect::<Vec<_>>().join(", ")
            ),
            Self::Object(o) => format!(
                "{{{}}}",
                o.iter()
                    .map(|(k, v)| format!("{}: {}", json_quote(k), v.json()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}
impl Index<&str> for Value {
    type Output = Self;
    fn index(&self, key: &str) -> &Self {
        if let Self::Object(o) = self {
            o.iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v)
                .unwrap_or(&Self::Null)
        } else {
            &Self::Null
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
    let text = format!("{f:?}");
    if let Some((mantissa, exponent)) = text.split_once('e') {
        let exponent: i32 = exponent.parse().expect("Rust formats a numeric exponent");
        format!("{mantissa}e{exponent:+03}")
    } else {
        text
    }
}
fn python_quote(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::from(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_control() || c.escape_debug().to_string().starts_with("\\u{") => {
                let n = c as u32;
                if n <= 0xff {
                    out.push_str(&format!("\\x{n:02x}"));
                } else if n <= 0xffff {
                    out.push_str(&format!("\\u{n:04x}"));
                } else {
                    out.push_str(&format!("\\U{n:08x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}
fn json_quote(s: &str) -> String {
    let quoted = serde_json::to_string(s).expect("a string serializes");
    let mut out = String::new();
    for c in quoted.chars() {
        if c as u32 >= 0x7f {
            for unit in c.encode_utf16(&mut [0; 2]) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        } else {
            out.push(c);
        }
    }
    out
}
