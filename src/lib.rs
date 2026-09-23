use sha1::{Digest, Sha1};
use wasm_bindgen::prelude::*;

/// SHA-1 digest of `data`, encoded as lowercase hex.
///
/// `data` is hashed as UTF-8 bytes, matching the historical wasm export.
pub fn sha1_hex(data: &str) -> String {
    let digest = Sha1::digest(data.as_bytes());
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[wasm_bindgen]
pub fn hash(data: &str) -> String {
    sha1_hex(data)
}

#[cfg(test)]
mod tests {
    use super::sha1_hex;

    #[test]
    fn empty_string() {
        assert_eq!(sha1_hex(""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }

    #[test]
    fn hello() {
        assert_eq!(
            sha1_hex("hello"),
            "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
        );
    }

    #[test]
    fn multibyte_utf8() {
        assert_eq!(
            sha1_hex("café"),
            "f424452a9673918c6f09b0cdd35b20be8e6ae7d7"
        );
    }
}
