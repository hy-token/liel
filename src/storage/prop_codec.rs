//! Binary encoding for property values on disk (`docs/reference/format-spec.ja.md`).
//!
//! - [`decode`]: Returns **only the first** complete value at the start of the slice;
//!   any trailing bytes are **ignored** (not an error). Callers that concatenate
//!   payloads must slice to exact boundaries if they need strict “no junk” checks.
//! - [`encode`] (including `Map`): Keys are sorted before writing so the wire bytes are
//!   **deterministic** across runs (see the `Map` arm in the private `encode_into` helper).
//!
//! Design trade-offs: `docs/design/product-tradeoffs.ja.md` §6.4.

use crate::error::{LielError, Result};
use std::collections::HashMap;

/// A dynamically typed property value that can be stored on a node or edge.
///
/// liel uses a custom binary encoding for all property values instead of a
/// third-party serialisation library (see `docs/design/product-tradeoffs.ja.md` §6.4).  Every
/// variant maps to a single-byte type tag followed by the payload bytes, as
/// described in `prop_codec` below.
///
/// # Supported types
/// | Variant           | Tag    | Wire size (excl. tag)                  |
/// |-------------------|--------|----------------------------------------|
/// | `Null`            | `0x00` | 0 bytes                                |
/// | `Bool(b)`         | `0x01` | 1 byte (`0x00` = false, `0x01` = true) |
/// | `Int(n)`          | `0x02` | 8 bytes, little-endian i64             |
/// | `Float(f)`        | `0x03` | 8 bytes, IEEE 754 little-endian f64    |
/// | `String(s)`       | `0x04` | 4-byte LE length + UTF-8 bytes         |
/// | `List(items)`     | `0x05` | 4-byte LE count + each element         |
/// | `Map(kv)`         | `0x06` | 4-byte LE count + (key + value) pairs  |
///
/// Map keys are *not* tagged: they are stored as `u32 length + UTF-8 bytes`
/// (the same as a `String` payload without the leading `0x04` tag).
#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<PropValue>),
    Map(HashMap<String, PropValue>),
}

// ─── Type tag constants ──────────────────────────────────────────────────────

/// Wire tag byte for `PropValue::Null` — zero bytes of payload follow.
const TAG_NULL: u8 = 0x00;
/// Wire tag byte for `PropValue::Bool` — one byte of payload follows.
const TAG_BOOL: u8 = 0x01;
/// Wire tag byte for `PropValue::Int` — eight bytes of LE i64 follow.
const TAG_INT: u8 = 0x02;
/// Wire tag byte for `PropValue::Float` — eight bytes of LE f64 (IEEE 754) follow.
const TAG_FLOAT: u8 = 0x03;
/// Wire tag byte for `PropValue::String` — four-byte LE length then UTF-8 bytes follow.
const TAG_STRING: u8 = 0x04;
/// Wire tag byte for `PropValue::List` — four-byte LE count then each element follows.
const TAG_LIST: u8 = 0x05;
/// Wire tag byte for `PropValue::Map` — four-byte LE count then (key, value) pairs follow.
///
/// Map keys are encoded without a tag byte: they use the `String` payload
/// format (4-byte length + UTF-8 content) directly.
const TAG_MAP: u8 = 0x06;

// ─── Public encoding API ─────────────────────────────────────────────────────

/// Encode a single `PropValue` into a freshly allocated `Vec<u8>`.
///
/// The encoding is self-describing: the returned bytes include the type tag
/// and all nested structure, so the full value can be recovered with
/// [`decode`] using only the byte slice.
///
/// # Parameters
/// - `value`: The property value to encode.
///
/// # Returns
/// A `Vec<u8>` holding the complete binary representation.  The length
/// depends on the variant and the size of any string/list/map contents.
pub fn encode(value: &PropValue) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_into(value, &mut buf);
    buf
}

/// Recursively encode `value` into `buf`, appending bytes without clearing
/// any existing content.
///
/// This is the internal workhorse called by both the public `encode` function
/// and itself recursively for `List` and `Map` elements.  Using a single
/// pre-allocated `Vec` avoids the overhead of many small allocations.
///
/// # Encoding decisions
/// - `Bool`: stored as a full byte (`0x01` or `0x00`) rather than packing bits,
///   for simplicity and alignment.
/// - `Float`: bits are reinterpreted as a `u64` via `to_bits()` then written
///   LE — this preserves NaN payloads faithfully.
/// - `String` length field is `u32` (max ~4 GiB per string), consistent with
///   the `List` and `Map` count fields.
/// - Map keys are written without a tag byte (their type is implicit: always
///   a UTF-8 string) to reduce wire size.
fn encode_into(value: &PropValue, buf: &mut Vec<u8>) {
    match value {
        PropValue::Null => {
            buf.push(TAG_NULL);
        }
        PropValue::Bool(b) => {
            buf.push(TAG_BOOL);
            buf.push(if *b { 0x01 } else { 0x00 });
        }
        PropValue::Int(n) => {
            buf.push(TAG_INT);
            buf.extend_from_slice(&n.to_le_bytes());
        }
        PropValue::Float(f) => {
            buf.push(TAG_FLOAT);
            // Use to_le_bytes() directly on f64; this preserves NaN bit patterns.
            buf.extend_from_slice(&f.to_le_bytes());
        }
        PropValue::String(s) => {
            buf.push(TAG_STRING);
            let bytes = s.as_bytes();
            // 4-byte little-endian length prefix, then raw UTF-8 bytes.
            buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(bytes);
        }
        PropValue::List(items) => {
            buf.push(TAG_LIST);
            // 4-byte little-endian element count.
            buf.extend_from_slice(&(items.len() as u32).to_le_bytes());
            for item in items {
                encode_into(item, buf);
            }
        }
        PropValue::Map(map) => {
            buf.push(TAG_MAP);
            // 4-byte little-endian entry count.
            buf.extend_from_slice(&(map.len() as u32).to_le_bytes());
            // Encode entries in ascending key order so the resulting byte
            // stream is deterministic.  `HashMap` has no defined iteration
            // order, which would otherwise make on-disk bytes differ between
            // runs even for identical input — breaking golden-file tests and
            // content-addressable diffs of `.liel` files.  The decoded Map
            // still uses `HashMap`, so the public API is unchanged.
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for key in keys {
                let val = &map[key];
                // Keys are encoded as raw String payloads: u32 length + UTF-8.
                // No tag byte is written for the key — its type is implicit.
                let key_bytes = key.as_bytes();
                buf.extend_from_slice(&(key_bytes.len() as u32).to_le_bytes());
                buf.extend_from_slice(key_bytes);
                // Values are fully-tagged PropValues.
                encode_into(val, buf);
            }
        }
    }
}

// ─── Public decoding API ─────────────────────────────────────────────────────

/// Decode a single `PropValue` from the beginning of `bytes`.
///
/// This is the inverse of [`encode`].  The full byte slice is consumed as a
/// single value; if there are trailing bytes after the first complete value,
/// they are silently ignored (the internal `decode_from` cursor stops after
/// the first value).
///
/// # Parameters
/// - `bytes`: A byte slice previously produced by `encode`.
///
/// # Errors
/// Returns `CorruptedFile` if the bytes are truncated, contain an unknown tag,
/// or contain invalid UTF-8 in a string or map key.
pub fn decode(bytes: &[u8]) -> Result<PropValue> {
    let (value, _) = decode_from(bytes, 0)?;
    Ok(value)
}

/// Recursively decode one `PropValue` starting at `pos` within `bytes`.
///
/// Returns the decoded value and the position immediately after the last byte
/// consumed.  The caller uses this updated position to continue decoding the
/// next element (e.g. the next item in a `List`).
///
/// # Parameters
/// - `bytes`: The full byte slice from which data is being read.
/// - `pos`: The byte index at which to start reading.
///
/// # Returns
/// A `(PropValue, usize)` tuple: the decoded value and the new cursor position.
///
/// # Errors
/// - `CorruptedFile("prop_codec: unexpected end of data")` if `pos >= bytes.len()`.
/// - `CorruptedFile("prop_codec: <type> truncated")` if a payload is cut off.
/// - `CorruptedFile("prop_codec: unknown tag 0x??")` for an unrecognised type tag.
/// - `CorruptedFile("prop_codec: invalid UTF-8")` for malformed string data.
fn decode_from(bytes: &[u8], pos: usize) -> Result<(PropValue, usize)> {
    if pos >= bytes.len() {
        return Err(LielError::CorruptedFile(
            "prop_codec: unexpected end of data".into(),
        ));
    }
    // Read the single-byte type tag and advance the cursor past it.
    let tag = bytes[pos];
    let pos = pos + 1;

    match tag {
        TAG_NULL => Ok((PropValue::Null, pos)),

        TAG_BOOL => {
            if pos >= bytes.len() {
                return Err(LielError::CorruptedFile(
                    "prop_codec: bool truncated".into(),
                ));
            }
            // Any non-zero byte is treated as `true`.
            Ok((PropValue::Bool(bytes[pos] != 0), pos + 1))
        }

        TAG_INT => {
            if pos + 8 > bytes.len() {
                return Err(LielError::CorruptedFile(
                    "prop_codec: int64 truncated".into(),
                ));
            }
            let n = i64::from_le_bytes(
                bytes[pos..pos + 8]
                    .try_into()
                    .expect("BUG: prop_codec slice indices bounds-checked above"),
            );
            Ok((PropValue::Int(n), pos + 8))
        }

        TAG_FLOAT => {
            if pos + 8 > bytes.len() {
                return Err(LielError::CorruptedFile(
                    "prop_codec: float64 truncated".into(),
                ));
            }
            // Re-interpret the 8 raw bytes as an IEEE 754 f64 bit pattern.
            let bits = u64::from_le_bytes(
                bytes[pos..pos + 8]
                    .try_into()
                    .expect("BUG: prop_codec slice indices bounds-checked above"),
            );
            Ok((PropValue::Float(f64::from_bits(bits)), pos + 8))
        }

        TAG_STRING => {
            if pos + 4 > bytes.len() {
                return Err(LielError::CorruptedFile(
                    "prop_codec: string length truncated".into(),
                ));
            }
            // Read the 4-byte LE length prefix.
            let len = u32::from_le_bytes(
                bytes[pos..pos + 4]
                    .try_into()
                    .expect("BUG: prop_codec slice indices bounds-checked above"),
            ) as usize;
            let pos = pos + 4;
            if pos + len > bytes.len() {
                return Err(LielError::CorruptedFile(
                    "prop_codec: string data truncated".into(),
                ));
            }
            // Validate and convert the UTF-8 payload.
            let s = std::str::from_utf8(&bytes[pos..pos + len])
                .map_err(|_| LielError::CorruptedFile("prop_codec: invalid UTF-8".into()))?
                .to_string();
            Ok((PropValue::String(s), pos + len))
        }

        TAG_LIST => {
            if pos + 4 > bytes.len() {
                return Err(LielError::CorruptedFile(
                    "prop_codec: list count truncated".into(),
                ));
            }
            // Read the 4-byte LE element count.
            let count = u32::from_le_bytes(
                bytes[pos..pos + 4]
                    .try_into()
                    .expect("BUG: prop_codec slice indices bounds-checked above"),
            ) as usize;
            let mut pos = pos + 4;
            let mut items = Vec::with_capacity(count);
            // Decode each element sequentially, advancing `pos` after each one.
            for _ in 0..count {
                let (item, next) = decode_from(bytes, pos)?;
                items.push(item);
                pos = next;
            }
            Ok((PropValue::List(items), pos))
        }

        TAG_MAP => {
            if pos + 4 > bytes.len() {
                return Err(LielError::CorruptedFile(
                    "prop_codec: map count truncated".into(),
                ));
            }
            // Read the 4-byte LE entry count.
            let count = u32::from_le_bytes(
                bytes[pos..pos + 4]
                    .try_into()
                    .expect("BUG: prop_codec slice indices bounds-checked above"),
            ) as usize;
            let mut pos = pos + 4;
            let mut map = HashMap::new();
            for _ in 0..count {
                // Decode the key: u32 length (LE) + UTF-8 bytes (no tag byte).
                if pos + 4 > bytes.len() {
                    return Err(LielError::CorruptedFile(
                        "prop_codec: map key length truncated".into(),
                    ));
                }
                let key_len = u32::from_le_bytes(
                    bytes[pos..pos + 4]
                        .try_into()
                        .expect("BUG: prop_codec slice indices bounds-checked above"),
                ) as usize;
                pos += 4;
                if pos + key_len > bytes.len() {
                    return Err(LielError::CorruptedFile(
                        "prop_codec: map key data truncated".into(),
                    ));
                }
                let key = std::str::from_utf8(&bytes[pos..pos + key_len])
                    .map_err(|_| {
                        LielError::CorruptedFile("prop_codec: invalid UTF-8 in key".into())
                    })?
                    .to_string();
                pos += key_len;
                // Decode the associated value (fully tagged PropValue).
                let (val, next) = decode_from(bytes, pos)?;
                map.insert(key, val);
                pos = next;
            }
            Ok((PropValue::Map(map), pos))
        }

        _ => Err(LielError::CorruptedFile(format!(
            "prop_codec: unknown tag 0x{:02x}",
            tag
        ))),
    }
}

// ─── Convenience helpers for the graph layer ─────────────────────────────────

/// Encode an entire property set (`HashMap<String, PropValue>`) into bytes.
///
/// The property set is treated as a `PropValue::Map` and encoded with
/// [`encode`].  This is the format stored in `NodeSlot::prop_offset` /
/// `EdgeSlot::prop_offset` regions.
///
/// # Parameters
/// - `props`: The property map to serialise.
///
/// # Returns
/// The binary encoding of the full map, beginning with the `0x06` Map tag.
pub fn encode_props(props: &HashMap<String, PropValue>) -> Vec<u8> {
    encode(&PropValue::Map(props.clone()))
}

/// Decode a property set from bytes previously produced by [`encode_props`].
///
/// # Parameters
/// - `bytes`: Raw bytes of a property blob read from an absolute file offset
///   (prop extents).
///
/// # Returns
/// The decoded `HashMap<String, PropValue>`.  An empty slice returns an empty
/// map (no error) because nodes and edges with no properties store nothing.
///
/// # Errors
/// Returns `CorruptedFile` if the bytes cannot be decoded or if the top-level
/// value is not a `Map`.
pub fn decode_props(bytes: &[u8]) -> Result<HashMap<String, PropValue>> {
    if bytes.is_empty() {
        // Nodes/edges with no properties store zero bytes; return an empty map.
        return Ok(HashMap::new());
    }
    match decode(bytes)? {
        PropValue::Map(m) => Ok(m),
        _ => Err(LielError::CorruptedFile("props: expected Map".into())),
    }
}

/// Encode a slice of label strings into bytes using the `List<String>` format.
///
/// Labels are stored as their own blob in property storage, separately from the property map.
/// A node can have multiple labels (e.g. `["Person", "Employee"]`); they are
/// encoded as a `PropValue::List` of `PropValue::String` values.
///
/// # Parameters
/// - `labels`: The ordered slice of label strings to encode.
///
/// # Returns
/// The binary encoding of the label list, beginning with the `0x05` List tag.
pub fn encode_labels(labels: &[String]) -> Vec<u8> {
    let list = PropValue::List(
        labels
            .iter()
            .map(|s| PropValue::String(s.clone()))
            .collect(),
    );
    encode(&list)
}

/// Decode a label list from bytes previously produced by [`encode_labels`].
///
/// # Parameters
/// - `bytes`: Raw bytes of a label-list blob read from an absolute file offset.
///
/// # Returns
/// The decoded `Vec<String>`.  An empty slice returns an empty vector.
///
/// # Errors
/// Returns `CorruptedFile` if the bytes cannot be decoded, if the top-level
/// value is not a `List`, or if any list element is not a `String`.
pub fn decode_labels(bytes: &[u8]) -> Result<Vec<String>> {
    if bytes.is_empty() {
        // A node with no labels stores zero bytes; return an empty vector.
        return Ok(Vec::new());
    }
    match decode(bytes)? {
        PropValue::List(items) => items
            .into_iter()
            .map(|item| match item {
                PropValue::String(s) => Ok(s),
                _ => Err(LielError::CorruptedFile(
                    "labels: expected String in list".into(),
                )),
            })
            .collect(),
        _ => Err(LielError::CorruptedFile("labels: expected List".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null() {
        let encoded = encode(&PropValue::Null);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, PropValue::Null);
    }

    #[test]
    fn test_bool_true() {
        let encoded = encode(&PropValue::Bool(true));
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, PropValue::Bool(true));
    }

    #[test]
    fn test_bool_false() {
        let encoded = encode(&PropValue::Bool(false));
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, PropValue::Bool(false));
    }

    #[test]
    fn test_int_zero() {
        let encoded = encode(&PropValue::Int(0));
        assert_eq!(decode(&encoded).unwrap(), PropValue::Int(0));
    }

    #[test]
    fn test_int_positive() {
        let encoded = encode(&PropValue::Int(42));
        assert_eq!(decode(&encoded).unwrap(), PropValue::Int(42));
    }

    #[test]
    fn test_int_negative() {
        let encoded = encode(&PropValue::Int(-1));
        assert_eq!(decode(&encoded).unwrap(), PropValue::Int(-1));
    }

    #[test]
    fn test_int_max() {
        let encoded = encode(&PropValue::Int(i64::MAX));
        assert_eq!(decode(&encoded).unwrap(), PropValue::Int(i64::MAX));
    }

    #[test]
    fn test_float() {
        let input = 314_f64 / 100.0;
        let encoded = encode(&PropValue::Float(input));
        match decode(&encoded).unwrap() {
            PropValue::Float(f) => assert!((f - input).abs() < 1e-10),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn test_float_nan() {
        let encoded = encode(&PropValue::Float(f64::NAN));
        match decode(&encoded).unwrap() {
            PropValue::Float(f) => assert!(f.is_nan()),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn test_string_empty() {
        let encoded = encode(&PropValue::String(String::new()));
        assert_eq!(decode(&encoded).unwrap(), PropValue::String(String::new()));
    }

    #[test]
    fn test_string_ascii() {
        let encoded = encode(&PropValue::String("hello".into()));
        assert_eq!(decode(&encoded).unwrap(), PropValue::String("hello".into()));
    }

    #[test]
    fn test_string_utf8() {
        let encoded = encode(&PropValue::String("café Δ".into()));
        assert_eq!(
            decode(&encoded).unwrap(),
            PropValue::String("café Δ".into())
        );
    }

    #[test]
    fn test_list_empty() {
        let encoded = encode(&PropValue::List(vec![]));
        assert_eq!(decode(&encoded).unwrap(), PropValue::List(vec![]));
    }

    #[test]
    fn test_list_mixed_types() {
        let list = PropValue::List(vec![
            PropValue::Int(1),
            PropValue::String("two".into()),
            PropValue::Bool(true),
        ]);
        let encoded = encode(&list);
        assert_eq!(decode(&encoded).unwrap(), list);
    }

    #[test]
    fn test_map_empty() {
        let map = PropValue::Map(HashMap::new());
        let encoded = encode(&map);
        assert_eq!(decode(&encoded).unwrap(), map);
    }

    #[test]
    fn test_map_multiple_entries() {
        let mut m = HashMap::new();
        m.insert("a".into(), PropValue::Int(1));
        m.insert("b".into(), PropValue::String("hello".into()));
        let map = PropValue::Map(m);
        let encoded = encode(&map);
        assert_eq!(decode(&encoded).unwrap(), map);
    }

    #[test]
    fn test_map_nested() {
        let mut inner_map = HashMap::new();
        inner_map.insert("x".into(), PropValue::Int(42));
        let list_with_map = PropValue::List(vec![PropValue::Map(inner_map)]);
        let mut outer = HashMap::new();
        outer.insert("nested".into(), list_with_map);
        let val = PropValue::Map(outer);
        let encoded = encode(&val);
        assert_eq!(decode(&encoded).unwrap(), val);
    }

    #[test]
    fn test_map_encoding_is_deterministic() {
        // Two equal Maps built with different insertion orders must produce
        // exactly the same byte stream.  HashMap iteration order would
        // otherwise vary between builds and platforms, so this guards the
        // key-sort in `encode_into`.
        let mut a = HashMap::new();
        a.insert("alpha".to_string(), PropValue::Int(1));
        a.insert("beta".to_string(), PropValue::Int(2));
        a.insert("gamma".to_string(), PropValue::Int(3));
        a.insert("delta".to_string(), PropValue::Int(4));

        let mut b = HashMap::new();
        b.insert("delta".to_string(), PropValue::Int(4));
        b.insert("gamma".to_string(), PropValue::Int(3));
        b.insert("alpha".to_string(), PropValue::Int(1));
        b.insert("beta".to_string(), PropValue::Int(2));

        let enc_a = encode(&PropValue::Map(a));
        let enc_b = encode(&PropValue::Map(b));
        assert_eq!(enc_a, enc_b, "Map encoding must be order-independent");
    }
}
