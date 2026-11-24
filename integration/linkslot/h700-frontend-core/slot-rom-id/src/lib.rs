#![no_std]

extern crate alloc;

use alloc::string::String;
use sha2::{Digest as _, Sha256};

/// Raw SHA-256 content identity used by Linkslot's local link protocol.
pub type Digest = [u8; 32];

/// Incremental SHA-256 hasher for VFS-backed ROM streams.
pub struct RomHasher(Sha256);

impl Default for RomHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl RomHasher {
    pub fn new() -> Self {
        Self(Sha256::new())
    }

    pub fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    pub fn finalize(self) -> Digest {
        self.0.finalize().into()
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    digest_hex(&digest_bytes(bytes))
}

pub fn digest_bytes(bytes: &[u8]) -> Digest {
    let mut hasher = RomHasher::new();
    hasher.update(bytes);
    hasher.finalize()
}

pub fn digest_hex(digest: &Digest) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn digest_from_hex(value: &str) -> Option<Digest> {
    if value.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex(pair[0])?;
        let low = hex(pair[1])?;
        out[index] = high * 16 + low;
    }
    Some(out)
}

fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

pub fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_the_standard_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn digest_hex_round_trips_in_both_cases() {
        let digest = digest_bytes(b"abc");
        let lower = digest_hex(&digest);
        let upper = lower.to_ascii_uppercase();
        assert_eq!(digest_from_hex(&lower), Some(digest));
        assert_eq!(digest_from_hex(&upper), Some(digest));
        assert!(is_sha256(&lower));
        assert!(is_sha256(&upper));
    }

    #[test]
    fn malformed_hex_is_rejected() {
        assert_eq!(digest_from_hex(""), None);
        assert_eq!(digest_from_hex("xyz"), None);
        assert_eq!(digest_from_hex(&"0".repeat(63)), None);
        assert_eq!(digest_from_hex(&"g".repeat(64)), None);
        assert!(!is_sha256(&"z".repeat(64)));
    }

    #[test]
    fn incremental_hashing_matches_single_buffer_hashing() {
        let bytes = b"Linkslot H700 no_std ROM identity";
        let expected = digest_bytes(bytes);
        let mut hasher = RomHasher::new();
        hasher.update(&bytes[..7]);
        hasher.update(&bytes[7..19]);
        hasher.update(&bytes[19..]);
        assert_eq!(hasher.finalize(), expected);
    }
}
