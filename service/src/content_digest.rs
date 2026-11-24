//! SHA-256 fingerprints of file contents (catalog inventory, glossary version).

use std::fmt::Write;

use sha2::{Digest, Sha256};

/// Lowercase hex SHA-256 of `bytes`, truncated to `bytes_kept` bytes of digest.
#[must_use]
pub fn sha256_hex(bytes: &[u8], bytes_kept: usize) -> String {
    Sha256::digest(bytes).iter().take(bytes_kept).fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    #[test]
    fn matches_known_digest() {
        assert_eq!(sha256_hex(b"abc", 4), "ba7816bf");
    }
}
