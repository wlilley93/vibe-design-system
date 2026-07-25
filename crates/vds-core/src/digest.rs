//! Digests, and the one normalisation used before taking any of them.
//!
//! VDS S-2(7): where a proof must compare two values it compares digests of the
//! normalised values, never the values. That is what keeps a pin a gate rather
//! than a store, so the digest helpers live in core and every comparison goes
//! through them.

use std::fmt;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::error::{Result, VdsError};

/// A `sha256:<64 lowercase hex>` digest.
///
/// A newtype rather than a `String` so that a digest and a free-text field
/// cannot be swapped by a mistyped struct literal, and so the prefix is
/// impossible to omit.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
#[schemars(regex(pattern = r"^sha256:[0-9a-f]{64}$"))]
pub struct Digest(String);

impl Digest {
    pub fn of_bytes(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Self(format!("sha256:{}", hex::encode(hasher.finalize())))
    }

    pub fn of_text(text: &str) -> Self {
        Self::of_bytes(text.as_bytes())
    }

    /// Parse a digest a caller supplied, refusing anything not in the one form.
    ///
    /// The newtype's whole value is that a `Digest` is known to be
    /// `sha256:<64 lowercase hex>`. Accepting a caller's string without checking
    /// it would put an arbitrary value behind that guarantee, and every later
    /// comparison against it would silently be a comparison against nonsense.
    pub fn parse(raw: &str) -> Result<Self> {
        let Some(hex) = raw.strip_prefix("sha256:") else {
            return Err(VdsError::precondition(format!(
                "{raw:?} is not a digest. The one form is `sha256:` followed by 64 lowercase \
                 hexadecimal characters."
            )));
        };
        if hex.len() != 64
            || !hex
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
        {
            return Err(VdsError::precondition(format!(
                "{raw:?} is not a digest: the part after `sha256:` is {} character(s) and must \
                 be 64 lowercase hexadecimal characters.",
                hex.len()
            )));
        }
        Ok(Self(raw.to_owned()))
    }

    /// Digest a file by streaming it, so a large asset does not have to be held
    /// in memory to be witnessed.
    pub fn of_file(path: &Path) -> Result<Self> {
        use std::io::Read;
        let mut file = std::fs::File::open(path).map_err(|e| VdsError::io(path.display(), e))?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|e| VdsError::io(path.display(), e))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Ok(Self(format!("sha256:{}", hex::encode(hasher.finalize()))))
    }

    /// Digest any serialisable structure through the one canonical form.
    pub fn of_value<T: Serialize>(value: &T) -> Result<Self> {
        Ok(Self::of_text(&canonical_json(value)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_well_formed(&self) -> bool {
        self.0.len() == 71
            && self.0.starts_with("sha256:")
            && self.0[7..]
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The one normalisation used before digesting any structure: JSON with sorted
/// keys, no insignificant whitespace, ASCII-escaped.
///
/// `serde_json::Value` is a `BTreeMap` under `preserve_order = false`, which is
/// the default, so a round trip through it sorts object keys. That is the whole
/// mechanism, and it is written down here because a digest whose normalisation
/// is implicit is a digest nobody can reproduce.
pub fn canonical_json<T: Serialize>(value: &T) -> Result<String> {
    let as_value = serde_json::to_value(value).map_err(|e| VdsError::Serialize {
        what: "value for digesting".into(),
        message: e.to_string(),
    })?;
    let mut out = Vec::new();
    write_canonical(&as_value, &mut out);
    Ok(String::from_utf8(out).expect("canonical json is ascii"))
}

fn write_canonical(value: &serde_json::Value, out: &mut Vec<u8>) {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            out.push(b'{');
            // BTreeMap iteration is already key-sorted; collect anyway so the
            // sort is stated rather than inherited from a feature flag.
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_json_string(key, out);
                out.push(b':');
                write_canonical(&map[*key], out);
            }
            out.push(b'}');
        }
        Value::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_canonical(item, out);
            }
            out.push(b']');
        }
        Value::String(s) => write_json_string(s, out),
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(true) => out.extend_from_slice(b"true"),
        Value::Bool(false) => out.extend_from_slice(b"false"),
        Value::Number(n) => out.extend_from_slice(n.to_string().as_bytes()),
    }
}

/// A JSON string with every non-ASCII scalar escaped, so the canonical form is
/// byte-identical whatever the platform's idea of a text file is.
fn write_json_string(s: &str, out: &mut Vec<u8>) {
    out.push(b'"');
    for ch in s.chars() {
        match ch {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\r' => out.extend_from_slice(b"\\r"),
            '\t' => out.extend_from_slice(b"\\t"),
            c if (c as u32) < 0x20 => {
                out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes())
            }
            c if c.is_ascii() => out.push(c as u8),
            c => {
                let mut buf = [0u16; 2];
                for unit in c.encode_utf16(&mut buf) {
                    out.extend_from_slice(format!("\\u{unit:04x}").as_bytes());
                }
            }
        }
    }
    out.push(b'"');
}

/// Digest an ordered set of `(path, digest)` rows. Used wherever "the digest of
/// a directory of files" is needed: the register, the declared surface, a pack.
pub fn digest_rows(rows: &[(String, Digest)]) -> Result<Digest> {
    let mut sorted: Vec<&(String, Digest)> = rows.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let flat: Vec<[&str; 2]> = sorted
        .iter()
        .map(|(path, digest)| [path.as_str(), digest.as_str()])
        .collect();
    Digest::of_value(&flat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_prefixed_and_lowercase_hex() {
        let d = Digest::of_text("x");
        assert!(d.is_well_formed(), "{d}");
        assert!(d.as_str().starts_with("sha256:"));
    }

    #[test]
    fn known_answer_matches_sha256_of_the_bytes() {
        // echo -n "" | sha256sum
        assert_eq!(
            Digest::of_text("").as_str(),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn canonical_json_sorts_keys_regardless_of_insertion_order() {
        #[derive(serde::Serialize)]
        struct A {
            z: u8,
            a: u8,
        }
        assert_eq!(
            canonical_json(&A { z: 1, a: 2 }).unwrap(),
            r#"{"a":2,"z":1}"#
        );
    }

    #[test]
    fn canonical_json_escapes_non_ascii_so_the_form_is_byte_stable() {
        // 'e' plus a combining acute, and an astral-plane character that must come
        // out as a surrogate pair. Neither may reach the digest as raw UTF-8, or the
        // same logical value digests differently depending on the writer.
        assert_eq!(canonical_json(&"e\u{301}").unwrap(), "\"e\\u0301\"");
        assert_eq!(canonical_json(&"\u{1f600}").unwrap(), "\"\\ud83d\\ude00\"");
    }

    #[test]
    fn canonical_json_escapes_control_characters_and_quotes() {
        assert_eq!(
            canonical_json(&"a\"b\\c\nd\te").unwrap(),
            "\"a\\\"b\\\\c\\nd\\te\""
        );
        assert_eq!(canonical_json(&"\u{7}").unwrap(), "\"\\u0007\"");
    }
    #[test]
    fn row_digest_is_order_independent() {
        let a = ("a.yaml".to_string(), Digest::of_text("1"));
        let b = ("b.yaml".to_string(), Digest::of_text("2"));
        assert_eq!(
            digest_rows(&[a.clone(), b.clone()]).unwrap(),
            digest_rows(&[b, a]).unwrap()
        );
    }

    #[test]
    fn row_digest_changes_when_a_row_changes() {
        let base = vec![("a.yaml".to_string(), Digest::of_text("1"))];
        let moved = vec![("a.yaml".to_string(), Digest::of_text("2"))];
        assert_ne!(digest_rows(&base).unwrap(), digest_rows(&moved).unwrap());
    }
}
