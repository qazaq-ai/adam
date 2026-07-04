//! # adam-seal — Protocol Seal
//!
//! Zero-dependency, pure-Rust cryptographic primitives used to turn a
//! briefing **допуск** protocol into a signed, publicly verifiable legal
//! artifact.  The whole signature path — SHA-256, SHA-512, and Ed25519
//! (RFC 8032) — is implemented in-tree so that a service inspector or a
//! court expert can audit exactly how a protocol is sealed, and can
//! verify a seal with any independent Ed25519 implementation.
//!
//! ## What this crate deliberately does *not* do
//!
//! No key-management hierarchy, no HSM integration, no rotation/revocation
//! ledger, no certificate chain.  Until a real deployment states its
//! requirements, a seal is one detached Ed25519 signature over a canonical
//! digest, with a per-operator key held outside the repository.  The seal
//! format carries an explicit algorithm tag, so a future implementation
//! can migrate without breaking already-issued protocols.
//!
//! ## Security note
//!
//! Correctness is gated on the official RFC 8032 known-answer vectors
//! (`tests/rfc8032.rs`).  The code is **not** constant-time; it is meant
//! for offline, on-device sealing where no remote timing oracle exists.

pub mod ed25519;
pub mod sha2;

pub use ed25519::{PUBLIC_KEY_LEN, SEED_LEN, SIGNATURE_LEN};
pub use sha2::{sha256, sha512};

/// Algorithm tag embedded in every seal so the format can evolve while
/// keeping already-issued protocols verifiable.
pub const SEAL_ALG: &str = "adam-ed25519-sha256-v1";

/// Lowercase-hex encoding of `bytes`.
pub fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

/// Decode a lowercase/uppercase hex string into bytes, or `None` if it is
/// not valid even-length hex.
pub fn from_hex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_val(bytes[i])?;
        let lo = hex_val(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Some(out)
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// An Ed25519 signing key: the 32-byte secret seed plus its derived
/// public key.  The seed is the private material and must be stored
/// outside the repository (e.g. `~/.config/adam/keys/<operator>.key`).
#[derive(Clone)]
pub struct SigningKey {
    seed: [u8; SEED_LEN],
    public: [u8; PUBLIC_KEY_LEN],
}

impl SigningKey {
    /// Build a signing key from a 32-byte secret seed.
    pub fn from_seed(seed: [u8; SEED_LEN]) -> Self {
        let public = ed25519::public_key(&seed);
        SigningKey { seed, public }
    }

    /// Parse a signing key from a hex-encoded 32-byte seed.
    pub fn from_seed_hex(hex: &str) -> Option<Self> {
        let bytes = from_hex(hex)?;
        if bytes.len() != SEED_LEN {
            return None;
        }
        let mut seed = [0u8; SEED_LEN];
        seed.copy_from_slice(&bytes);
        Some(Self::from_seed(seed))
    }

    /// The 32-byte public verification key.
    pub fn public_key(&self) -> [u8; PUBLIC_KEY_LEN] {
        self.public
    }

    /// Hex-encoded public key.
    pub fn public_key_hex(&self) -> String {
        to_hex(&self.public)
    }

    /// Hex-encoded secret seed (handle with care — this is private).
    pub fn seed_hex(&self) -> String {
        to_hex(&self.seed)
    }

    /// Sign `msg`, returning a 64-byte detached signature.
    pub fn sign(&self, msg: &[u8]) -> [u8; SIGNATURE_LEN] {
        ed25519::sign(&self.seed, msg)
    }
}

/// Verify a detached signature (hex) over `msg` against a public key (hex).
/// Returns `false` on any malformed input rather than erroring.
pub fn verify_hex(public_hex: &str, msg: &[u8], sig_hex: &str) -> bool {
    let (Some(pk), Some(sig)) = (from_hex(public_hex), from_hex(sig_hex)) else {
        return false;
    };
    if pk.len() != PUBLIC_KEY_LEN || sig.len() != SIGNATURE_LEN {
        return false;
    }
    let mut pk_arr = [0u8; PUBLIC_KEY_LEN];
    let mut sig_arr = [0u8; SIGNATURE_LEN];
    pk_arr.copy_from_slice(&pk);
    sig_arr.copy_from_slice(&sig);
    ed25519::verify(&pk_arr, msg, &sig_arr)
}

/// Generate a fresh signing key from operating-system entropy.
///
/// Reads 32 bytes from the OS CSPRNG (`/dev/urandom` on Unix).  This is
/// the only place the crate touches the outside world, and only on the
/// key-generation path — sealing and verification are pure functions.
#[cfg(unix)]
pub fn generate_signing_key() -> std::io::Result<SigningKey> {
    use std::io::Read;
    let mut seed = [0u8; SEED_LEN];
    let mut f = std::fs::File::open("/dev/urandom")?;
    f.read_exact(&mut seed)?;
    Ok(SigningKey::from_seed(seed))
}
