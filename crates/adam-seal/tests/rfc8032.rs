//! Known-answer tests.  A pure-Rust Ed25519 is only trustworthy if it
//! reproduces the official RFC 8032 §7.1 vectors bit-for-bit, so these
//! are the release gate for the whole seal path.  The vectors are quoted
//! verbatim from RFC 8032 (Ed25519); the SHA-2 vectors are from FIPS 180-4.

use adam_seal::{SigningKey, ed25519, from_hex, sha256, sha512, to_hex, verify_hex};

fn seed_of(hex: &str) -> [u8; 32] {
    let mut s = [0u8; 32];
    s.copy_from_slice(&from_hex(hex).unwrap());
    s
}

/// (secret, public, message, signature) — RFC 8032 §7.1.
const VECTORS: &[(&str, &str, &str, &str)] = &[
    (
        "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
        "",
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
    ),
    (
        "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
        "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
        "72",
        "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
    ),
    (
        "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7",
        "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025",
        "af82",
        "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a",
    ),
    (
        "833fe62409237b9d62ec77587520911e9a759cec1d19755b7da901b96dca3d42",
        "ec172b93ad5e563bf4932c70e1245034c35467ef2efd4d64ebf819683467e2bf",
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
        "dc2a4459e7369633a52b1bf277839a00201009a3efbf3ecb69bea2186c26b58909351fc9ac90b3ecfdfbc7c66431e0303dca179c138ac17ad9bef1177331a704",
    ),
];

#[test]
fn ed25519_public_key_matches_rfc8032() {
    for (sk, pk, _msg, _sig) in VECTORS {
        let got = ed25519::public_key(&seed_of(sk));
        assert_eq!(to_hex(&got), *pk, "public key mismatch for seed {sk}");
    }
}

#[test]
fn ed25519_sign_matches_rfc8032() {
    for (sk, _pk, msg, sig) in VECTORS {
        let m = from_hex(msg).unwrap();
        let got = ed25519::sign(&seed_of(sk), &m);
        assert_eq!(to_hex(&got), *sig, "signature mismatch for message {msg}");
    }
}

#[test]
fn ed25519_verify_accepts_rfc8032() {
    for (_sk, pk, msg, sig) in VECTORS {
        let m = from_hex(msg).unwrap();
        assert!(
            verify_hex(pk, &m, sig),
            "verify rejected valid vector {sig}"
        );
    }
}

#[test]
fn ed25519_verify_rejects_tampered_message() {
    let (_sk, pk, _msg, sig) = VECTORS[1];
    // Original message is "72"; flip it.
    assert!(
        !verify_hex(pk, &[0x73], sig),
        "verify accepted a forged message"
    );
}

#[test]
fn ed25519_verify_rejects_tampered_signature() {
    let (_sk, pk, msg, sig) = VECTORS[2];
    let m = from_hex(msg).unwrap();
    let mut bad = from_hex(sig).unwrap();
    bad[0] ^= 0x01;
    assert!(
        !verify_hex(pk, &m, &to_hex(&bad)),
        "verify accepted a mutated signature"
    );
}

#[test]
fn ed25519_roundtrip_via_signing_key() {
    let key = SigningKey::from_seed(seed_of(VECTORS[0].0));
    let msg = b"OT/TB dopusk protocol";
    let sig = key.sign(msg);
    assert!(verify_hex(&key.public_key_hex(), msg, &to_hex(&sig)));
    // Wrong message must fail.
    assert!(!verify_hex(
        &key.public_key_hex(),
        b"tampered",
        &to_hex(&sig)
    ));
}

#[test]
fn sha256_fips_vectors() {
    assert_eq!(
        to_hex(&sha256(b"")),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        to_hex(&sha256(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        to_hex(&sha256(
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
        )),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn sha512_fips_vectors() {
    assert_eq!(
        to_hex(&sha512(b"")),
        "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce\
         47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
    );
    assert_eq!(
        to_hex(&sha512(b"abc")),
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
         2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
    );
}
