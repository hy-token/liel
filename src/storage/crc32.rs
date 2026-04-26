//! CRC32/ISO-HDLC checksum — zero external dependencies.
//!
//! This module provides the same CRC32 variant as the `crc32fast` crate
//! (polynomial 0xEDB88320, reflected bit order, initial value 0xFFFFFFFF,
//! final XOR 0xFFFFFFFF).  It is used by the WAL layer to detect bit-flip
//! corruption in WAL entries before they are applied to data pages.
//!
//! # Implementation
//! The lookup table is computed at **compile time** using a `const fn`, so
//! there is zero runtime initialisation cost.  The actual `crc32` function
//! is a straightforward byte-by-byte table walk that processes one byte per
//! iteration — perfectly adequate for WAL entries (≤ 4113 bytes each).

/// Pre-computed CRC32/ISO-HDLC lookup table.
///
/// `CRC_TABLE[i]` is the CRC32 of the single byte `i`.  Every entry is
/// derived from the reflected polynomial 0xEDB88320 using `make_crc_table`.
const CRC_TABLE: [u32; 256] = make_crc_table();

/// Build the 256-entry CRC32 lookup table at compile time.
///
/// For each byte value `i` (0–255) the function runs 8 rounds of the CRC
/// shift-register update:
/// - If the LSB is set, XOR with the reflected polynomial 0xEDB88320.
/// - Otherwise just shift right by one.
///
/// This produces the CRC32 of a one-byte message whose value is `i`.
const fn make_crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut c = i as u32;
        let mut bit = 0usize;
        while bit < 8 {
            if c & 1 != 0 {
                // Bit is set: XOR with reflected polynomial
                c = 0xEDB8_8320 ^ (c >> 1);
            } else {
                // Bit is clear: just shift
                c >>= 1;
            }
            bit += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

/// Compute the CRC32/ISO-HDLC checksum of `data`.
///
/// Produces the same result as `crc32fast::hash(data)`:
/// initial CRC = 0xFFFFFFFF, process each byte through the table, final
/// XOR with 0xFFFFFFFF.
///
/// # Parameters
/// - `data`: Arbitrary byte slice to checksum (may be empty).
///
/// # Returns
/// A 32-bit CRC value.  Two identical byte slices always produce the same
/// value; any single-bit difference will (with very high probability) produce
/// a different value.
///
/// # Example
/// ```ignore
/// let crc = crc32(b"hello");
/// assert_eq!(crc, 0x3610_A686);
/// ```
pub fn crc32(data: &[u8]) -> u32 {
    // Start with all-ones initial value (standard for CRC32/ISO-HDLC).
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        // XOR the low byte of the current CRC with the input byte, use as
        // table index, then shift the CRC right and XOR with the table entry.
        let index = ((crc ^ u32::from(byte)) & 0xFF) as usize;
        crc = CRC_TABLE[index] ^ (crc >> 8);
    }
    // Final XOR with all-ones produces the standard CRC32 output.
    crc ^ 0xFFFF_FFFF
}

#[cfg(test)]
mod tests {
    use super::crc32;

    /// CRC32 of an empty slice is the fixed value 0x00000000 for this variant.
    #[test]
    fn test_empty() {
        assert_eq!(crc32(b""), 0x0000_0000);
    }

    /// Well-known test vector: CRC32("123456789") == 0xCBF43926.
    #[test]
    fn test_known_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    /// Two different byte slices must not collide (simple sanity check).
    #[test]
    fn test_distinct_inputs() {
        assert_ne!(crc32(b"hello"), crc32(b"world"));
    }

    /// A single-bit difference in input must change the CRC.
    #[test]
    fn test_single_bit_sensitivity() {
        let a = crc32(b"\x00");
        let b = crc32(b"\x01");
        assert_ne!(a, b);
    }
}
