// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! Authenticated encryption, behind `மறை` and `வெளிப்படு`.
//!
//! ## Why this module looks like this
//!
//! There was a cipher here before: a repeating-key XOR with a default key. It
//! was unreachable — no statement, builtin or bytecode ever called it, so no
//! eTamil program could encrypt anything — and it was deleted rather than
//! improved, because dead code that looks like a security feature invites
//! someone to wire it up.
//!
//! Nothing about the replacement is invented. Every primitive is a vetted
//! implementation from the RustCrypto project, and the composition is the
//! ordinary one:
//!
//! * **XChaCha20-Poly1305** for the encryption itself — an AEAD, so tampering
//!   is *detected* rather than decrypted into plausible rubbish. The X variant
//!   takes a 24-byte nonce instead of 12. That matters here because the nonce
//!   is random per message rather than a counter: 24 bytes makes a collision a
//!   non-question, where 12 would need an argument about how many messages one
//!   key may encrypt.
//! * **Argon2id** to turn a passphrase into a key. A user types a password, not
//!   thirty-two random bytes, and a plain hash would let an attacker try
//!   billions of guesses a second against a stolen file. Argon2id is the
//!   current recommendation and the crate's default parameters are the OWASP
//!   baseline (19 MiB, two passes).
//! * A **random salt per message**, so the same passphrase over the same
//!   plaintext never produces the same output, and a precomputed table is worth
//!   nothing.
//!
//! ## The format
//!
//! One base64 string, so the result is an ordinary eTamil `சொல்` that can be
//! written with `கோப்பு_எழுது` or stored in a database column:
//!
//! ```text
//! version │ salt     │ nonce    │ ciphertext ‖ tag
//! 1 byte  │ 16 bytes │ 24 bytes │ n bytes    ‖ 16 bytes
//! ```
//!
//! The version byte is first and is **authenticated as associated data**, so it
//! cannot be flipped to talk a future reader into the wrong format. Reading a
//! version this build does not know is an error naming the version, which is
//! the difference between "I cannot read this" and a wrong answer.
//!
//! ## What it does not do
//!
//! It encrypts a *value*, not a stream. The whole plaintext and the whole
//! ciphertext are in memory at once, so this is for a document, a row, a
//! configuration file — not a ten-gigabyte archive. Chunking would need a
//! framing format and a per-chunk counter, and it is not needed by anything
//! yet.

use argon2::Argon2;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use zeroize::Zeroize;

/// The only format this build writes. Stored first and authenticated, so a
/// later version can refuse an older file loudly instead of misreading it.
const VERSION: u8 = 1;

const SALT: usize = 16;
const NONCE: usize = 24;
const KEY: usize = 32;

/// Where each part begins, named rather than counted at the call sites.
const SALT_AT: usize = 1;
const NONCE_AT: usize = SALT_AT + SALT;
const BODY_AT: usize = NONCE_AT + NONCE;

/// Poly1305's tag. A body shorter than this cannot be a message at all, so it
/// is rejected before the cipher is asked.
const TAG: usize = 16;

fn random(bytes: &mut [u8]) -> Result<(), String> {
    getrandom::fill(bytes).map_err(|_| {
        "இயந்திரத்தின் சீரற்ற எண் ஆதாரம் கிடைக்கவில்லை  \
         (no source of randomness available, so a key cannot be made safely)"
            .to_string()
    })
}

/// Argon2id over the passphrase and salt. The derived key is returned in a
/// buffer the caller wipes.
fn derive(passphrase: &str, salt: &[u8]) -> Result<[u8; KEY], String> {
    let mut key = [0u8; KEY];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|why| format!("விசை உருவாக்க முடியவில்லை  (could not derive a key: {})", why))?;
    Ok(key)
}

/// `மறை(உரை, கடவுச்சொல்)` — the base64 of everything a reader needs.
pub fn encrypt(plaintext: &str, passphrase: &str) -> Result<String, String> {
    if passphrase.is_empty() {
        return Err("கடவுச்சொல் காலியாக இருக்கக்கூடாது  \
                    (the passphrase must not be empty)"
            .to_string());
    }

    let mut salt = [0u8; SALT];
    random(&mut salt)?;
    let mut nonce = [0u8; NONCE];
    random(&mut nonce)?;

    let mut key = derive(passphrase, &salt)?;
    let cipher = XChaCha20Poly1305::new(&Key::from(key));
    // The buffer this module owns is wiped. The expanded state inside the
    // cipher is the cipher's business and outlives this line.
    key.zeroize();

    let sealed = cipher
        .encrypt(
            &XNonce::from(nonce),
            Payload {
                msg: plaintext.as_bytes(),
                aad: &[VERSION],
            },
        )
        .map_err(|_| "மறையாக்கம் தோல்வி  (encryption failed)".to_string())?;

    let mut out = Vec::with_capacity(BODY_AT + sealed.len());
    out.push(VERSION);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&sealed);
    Ok(BASE64.encode(out))
}

/// `வெளிப்படு(மறையீடு, கடவுச்சொல்)` — the plaintext, or why not.
///
/// Every failure is one message: a wrong passphrase and a tampered byte are
/// indistinguishable to the caller, and saying which would tell an attacker
/// whether a guess was close.
pub fn decrypt(encoded: &str, passphrase: &str) -> Result<String, String> {
    let raw = BASE64
        .decode(encoded.trim().as_bytes())
        .map_err(|_| "இது மறையாக்கப்பட்ட உரை அல்ல  (this is not encrypted text)".to_string())?;

    if raw.len() < BODY_AT + TAG {
        return Err("மறையாக்கப்பட்ட உரை முழுமையற்றது  (the encrypted text is truncated)".to_string());
    }

    let version = raw[0];
    if version != VERSION {
        return Err(format!(
            "பதிப்பு {} தெரியாது  (unknown format version {}: this build writes and reads {})",
            version, version, VERSION
        ));
    }

    let mut salt = [0u8; SALT];
    salt.copy_from_slice(&raw[SALT_AT..NONCE_AT]);
    let mut nonce = [0u8; NONCE];
    nonce.copy_from_slice(&raw[NONCE_AT..BODY_AT]);

    let mut key = derive(passphrase, &salt)?;
    let cipher = XChaCha20Poly1305::new(&Key::from(key));
    key.zeroize();

    let opened = cipher
        .decrypt(
            &XNonce::from(nonce),
            Payload {
                msg: &raw[BODY_AT..],
                aad: &[version],
            },
        )
        .map_err(|_| {
            "கடவுச்சொல் தவறு, அல்லது உரை மாற்றப்பட்டுள்ளது  \
             (wrong passphrase, or the text has been altered)"
                .to_string()
        })?;

    String::from_utf8(opened)
        .map_err(|_| "மறைநீக்கிய தரவு உரை அல்ல  (the decrypted data is not text)".to_string())
}

/// `மறை_விசை()` — a fresh passphrase nobody has to invent.
///
/// Thirty-two random bytes as base64. It goes in the same argument slot as a
/// typed passphrase, because a 44-character random string simply *is* a very
/// good one — which keeps one format and one code path rather than a second
/// "raw key" mode to get wrong.
pub fn fresh_key() -> Result<String, String> {
    let mut key = [0u8; KEY];
    random(&mut key)?;
    let encoded = BASE64.encode(key);
    key.zeroize();
    Ok(encoded)
}

/// SHA-256 of `message`, as the digest's own bytes.
///
/// Bytes rather than hex, because what a caller does next is almost always
/// encode them and hex would have to be undone first. PKCE, for one, wants
/// base64url of the digest. `nUlakam/vativam/kuRiyAkkam.qmz` encodes an array
/// of bytes in eTamil, so the host stops where the language actually stops: a
/// digest is bit operations, and eTamil has none.
pub fn sha256(message: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    Sha256::digest(message.as_bytes()).to_vec()
}

/// `count` bytes from the operating system's randomness.
///
/// There was no way to obtain an unguessable value from eTamil before this. A
/// program needing one — an OAuth `state`, a `nonce`, a PKCE verifier — had the
/// clock and nothing else, and a value derived from the clock is a value an
/// attacker can derive too.
pub fn random_bytes(count: usize) -> Result<Vec<u8>, String> {
    let mut buffer = vec![0u8; count];
    random(&mut buffer)?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_round_trip_returns_what_went_in() {
        let sealed = encrypt("வணக்கம், ₹2.05", "correct horse").unwrap();
        assert_eq!(decrypt(&sealed, "correct horse").unwrap(), "வணக்கம், ₹2.05");
    }

    #[test]
    fn the_same_text_twice_does_not_look_the_same() {
        // A fresh salt and nonce per message, so identical plaintext under an
        // identical passphrase gives different output. Without this, a ledger
        // full of repeated amounts would leak which rows match.
        let one = encrypt("1000", "k").unwrap();
        let two = encrypt("1000", "k").unwrap();
        assert_ne!(one, two);
        assert_eq!(decrypt(&one, "k").unwrap(), "1000");
        assert_eq!(decrypt(&two, "k").unwrap(), "1000");
    }

    #[test]
    fn a_wrong_passphrase_is_refused_rather_than_guessed_at() {
        let sealed = encrypt("secret", "right").unwrap();
        let why = decrypt(&sealed, "wrong").unwrap_err();
        assert!(why.contains("wrong passphrase"), "{}", why);
    }

    #[test]
    fn a_tampered_byte_is_detected() {
        // The point of an AEAD. A stream cipher without a tag would hand back
        // altered plaintext and say nothing.
        let sealed = encrypt("₹1,00,000 to A", "k").unwrap();
        let mut raw = BASE64.decode(&sealed).unwrap();
        let last = raw.len() - 1;
        raw[last] ^= 0x01;
        let why = decrypt(&BASE64.encode(raw), "k").unwrap_err();
        assert!(
            why.contains("altered") || why.contains("wrong passphrase"),
            "{}",
            why
        );
    }

    #[test]
    fn flipping_the_version_byte_is_detected() {
        // It is authenticated as associated data, so it cannot be edited to
        // steer a future reader into the wrong format.
        let sealed = encrypt("x", "k").unwrap();
        let mut raw = BASE64.decode(&sealed).unwrap();
        raw[0] = 99;
        let why = decrypt(&BASE64.encode(raw), "k").unwrap_err();
        assert!(why.contains("unknown format version 99"), "{}", why);
    }

    #[test]
    fn truncation_is_refused_before_the_cipher_is_asked() {
        let sealed = encrypt("x", "k").unwrap();
        let raw = BASE64.decode(&sealed).unwrap();
        let short = BASE64.encode(&raw[..BODY_AT + 2]);
        let why = decrypt(&short, "k").unwrap_err();
        assert!(why.contains("truncated"), "{}", why);
    }

    #[test]
    fn text_that_was_never_encrypted_says_so() {
        let why = decrypt("this is just a sentence", "k").unwrap_err();
        assert!(why.contains("not encrypted text"), "{}", why);
    }

    #[test]
    fn an_empty_passphrase_is_refused() {
        // Encrypting under no passphrase at all is the mistake the old default
        // key made permanent.
        let why = encrypt("x", "").unwrap_err();
        assert!(why.contains("must not be empty"), "{}", why);
    }

    #[test]
    fn a_generated_key_is_a_usable_passphrase() {
        let key = fresh_key().unwrap();
        assert_ne!(key, fresh_key().unwrap());
        let sealed = encrypt("₹42.50", &key).unwrap();
        assert_eq!(decrypt(&sealed, &key).unwrap(), "₹42.50");
    }

    #[test]
    fn an_empty_plaintext_round_trips() {
        let sealed = encrypt("", "k").unwrap();
        assert_eq!(decrypt(&sealed, "k").unwrap(), "");
    }
}
