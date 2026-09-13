//! Lossless Python strings, including escaped lone UTF-16 surrogates.
//!
//! Rust formatting only carries UTF-8. Internally we transport NUL as two NULs and an unpaired
//! surrogate as NUL + 'u' + four hex digits. Every input NUL is escaped, so literal input cannot
//! collide with the transport. Report serialization and stdout encoding decode it explicitly.
use serde::{Serialize, Serializer};
use std::fmt;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Text(String);
impl Text {
    pub(crate) fn from_encoded(text: String) -> Self {
        Self(text)
    }
    pub(crate) fn scalar(text: &str) -> Self {
        let mut out = Self::default();
        for c in text.chars() {
            out.push(c as u32);
        }
        out
    }
    pub(crate) fn push(&mut self, point: u32) {
        match point {
            0 => self.0.push_str("\0\0"),
            0xd800..=0xdfff => self.0.push_str(&format!("\0u{point:04x}")),
            _ => self
                .0
                .push(char::from_u32(point).expect("parser produced a Unicode codepoint")),
        }
    }
    pub(crate) fn encoded(&self) -> &str {
        &self.0
    }
    pub(crate) fn points(&self) -> impl Iterator<Item = u32> + '_ {
        let mut chars = self.0.chars();
        std::iter::from_fn(move || {
            let c = chars.next()?;
            if c != '\0' {
                return Some(c as u32);
            }
            match chars.next().expect("complete text transport") {
                '\0' => Some(0),
                'u' => {
                    let mut value = 0;
                    for _ in 0..4 {
                        value = value * 16
                            + chars
                                .next()
                                .and_then(|c| c.to_digit(16))
                                .expect("surrogate transport");
                    }
                    Some(value)
                }
                _ => unreachable!("all input NULs are escaped"),
            }
        })
    }
    pub(crate) fn characters(&self) -> Vec<String> {
        self.points()
            .map(|point| {
                let mut t = Self::default();
                t.push(point);
                t.0
            })
            .collect()
    }
    pub(crate) fn json(&self) -> String {
        let mut out = String::from("\"");
        for point in self.points() {
            match point {
                0x22 => out.push_str("\\\""),
                0x5c => out.push_str("\\\\"),
                8 => out.push_str("\\b"),
                12 => out.push_str("\\f"),
                10 => out.push_str("\\n"),
                13 => out.push_str("\\r"),
                9 => out.push_str("\\t"),
                0x20..=0x7e => out.push(char::from_u32(point).unwrap()),
                0..=0xffff => out.push_str(&format!("\\u{point:04x}")),
                _ => {
                    let n = point - 0x10000;
                    out.push_str(&format!(
                        "\\u{:04x}\\u{:04x}",
                        0xd800 + (n >> 10),
                        0xdc00 + (n & 0x3ff)
                    ));
                }
            }
        }
        out.push('"');
        out
    }
    /// UTF-8 with the legacy CLI's surrogateescape error policy. Other lone surrogates fail
    /// before any stdout bytes are written; U+DC80..U+DCFF round-trip as their original bytes.
    pub fn stdout_bytes(&self) -> Result<Vec<u8>, String> {
        let mut out = vec![];
        for point in self.points() {
            match point {
                0xdc80..=0xdcff => out.push((point - 0xdc00) as u8),
                0xd800..=0xdfff => {
                    return Err(format!(
                        "cannot encode unpaired surrogate U+{point:04X} as UTF-8"
                    ));
                }
                _ => {
                    let mut buffer = [0; 4];
                    out.extend_from_slice(
                        char::from_u32(point)
                            .unwrap()
                            .encode_utf8(&mut buffer)
                            .as_bytes(),
                    );
                }
            }
        }
        Ok(out)
    }
}
impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.encoded())
    }
}
impl Serialize for Text {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let raw = serde_json::value::RawValue::from_string(self.json())
            .map_err(serde::ser::Error::custom)?;
        raw.serialize(serializer)
    }
}
