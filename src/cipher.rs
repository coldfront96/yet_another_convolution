//! The encoded outer layer — the "only my repos understand it" gimmick.
//!
//! A `.wild` program can be wrapped so that on disk it looks like meaningless
//! base64 garbage. The CLI auto-detects the wrapper, decodes it with a key, and
//! runs the revealed source. Without the right key the payload is gibberish.
//!
//! Format of an encoded file:
//!
//! ```text
//! CNVL1:<base64 of XOR-ciphertext>
//! ```
//!
//! The ciphertext is the plaintext (prefixed with a small inner tag) XORed with
//! a keystream derived from the key, then base64-encoded. The inner tag lets a
//! wrong key fail loudly with [`CipherError::WrongKey`] instead of silently
//! running garbage.
//!
//! This is obfuscation, not real cryptography — a keystream XOR is exactly as
//! strong as keeping the key secret and no stronger. That is the right strength
//! for a "for fun" esolang; do not protect anything that actually matters with
//! it.

use std::fmt;

/// Marks a file as Convolution-encoded, version 1.
pub const MAGIC: &str = "CNVL1:";

/// A short tag prepended to the plaintext before encryption so that decoding
/// with the wrong key can be detected.
const INNER_TAG: &[u8] = b"WILD\x01";

const B64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Something went wrong while decoding an encoded program.
#[derive(Debug, PartialEq, Eq)]
pub enum CipherError {
    /// The content didn't start with the [`MAGIC`] marker.
    MissingMagic,
    /// The base64 payload was malformed.
    BadBase64,
    /// The payload decoded, but the inner tag didn't match — almost always the
    /// wrong key.
    WrongKey,
}

impl fmt::Display for CipherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CipherError::MissingMagic => {
                write!(f, "cipher error: not an encoded program (missing `{MAGIC}` marker)")
            }
            CipherError::BadBase64 => write!(f, "cipher error: corrupt base64 payload"),
            CipherError::WrongKey => {
                write!(f, "cipher error: wrong key (the decoded payload failed its check)")
            }
        }
    }
}

/// True if `s` looks like an encoded program (ignoring leading whitespace).
pub fn looks_encoded(s: &str) -> bool {
    s.trim_start().starts_with(MAGIC)
}

/// Encode `plaintext` source into a self-describing encoded string.
pub fn encode(plaintext: &str, key: &str) -> String {
    let mut buf = INNER_TAG.to_vec();
    buf.extend_from_slice(plaintext.as_bytes());
    xor_keystream(&mut buf, key);
    format!("{MAGIC}{}", b64_encode(&buf))
}

/// Decode an encoded string back into source, verifying the key.
pub fn decode(encoded: &str, key: &str) -> Result<String, CipherError> {
    let trimmed = encoded.trim();
    let payload = trimmed
        .strip_prefix(MAGIC)
        .ok_or(CipherError::MissingMagic)?;

    let mut bytes = b64_decode(payload)?;
    xor_keystream(&mut bytes, key);

    let body = bytes
        .strip_prefix(INNER_TAG)
        .ok_or(CipherError::WrongKey)?;

    // After a correct key the tag matched, so the remainder is the original
    // UTF-8 source; treat any oddity here as a wrong key too.
    String::from_utf8(body.to_vec()).map_err(|_| CipherError::WrongKey)
}

// --- keystream ----------------------------------------------------------

/// XOR `data` in place with a keystream derived from `key`.
fn xor_keystream(data: &mut [u8], key: &str) {
    let mut ks = KeyStream::new(key);
    for b in data.iter_mut() {
        *b ^= ks.next_byte();
    }
}

/// A deterministic byte generator seeded from the key (FNV-1a seed, SplitMix64
/// output). Keeps the cipher dependency-free.
struct KeyStream {
    state: u64,
}

impl KeyStream {
    fn new(key: &str) -> Self {
        // FNV-1a 64-bit hash of the key.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in key.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        KeyStream { state: h }
    }

    fn next_byte(&mut self) -> u8 {
        // SplitMix64.
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z & 0xFF) as u8
    }
}

// --- base64 (dependency-free) -------------------------------------------

fn b64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);

        out.push(B64_ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(B64_ALPHABET[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            B64_ALPHABET[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64_ALPHABET[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn b64_decode(s: &str) -> Result<Vec<u8>, CipherError> {
    // Collect 6-bit values, ignoring padding and whitespace.
    let mut sextets: Vec<u8> = Vec::with_capacity(s.len());
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b'\n' | b'\r' | b' ' | b'\t' => continue,
            _ => return Err(CipherError::BadBase64),
        };
        sextets.push(v);
    }

    let mut out = Vec::with_capacity(sextets.len() / 4 * 3);
    for chunk in sextets.chunks(4) {
        if chunk.len() < 2 {
            return Err(CipherError::BadBase64);
        }
        let n = ((chunk[0] as u32) << 18)
            | ((chunk[1] as u32) << 12)
            | ((chunk.get(2).copied().unwrap_or(0) as u32) << 6)
            | (chunk.get(3).copied().unwrap_or(0) as u32);
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    Ok(out)
}
