// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! ECDSA over P-256 — signatures that a bank or a ledger will accept.
//!
//! HMAC, which this crate already had, proves a message came from someone who
//! holds the same secret you do. That is enough for a webhook and not enough
//! for anything else: both sides can forge each other's messages, so neither
//! can prove to a third party which of them sent one.
//!
//! A signature here is made with a private key and checked with a public one,
//! so the holder of the key is the only party who could have produced it. That
//! is what Hyperledger Fabric requires of an MSP identity, and what a bank
//! requires of a request that moves money.
//!
//! P-256 with SHA-256 specifically, because that is what Fabric's default MSP
//! uses and what most Indian banking APIs specify.
//!
//! Keys and signatures cross into the language as lowercase hex, the same way
//! HMAC signatures already do — a private key is the 32-byte scalar, a public
//! key the 65-byte uncompressed SEC1 point, and a signature is ASN.1 DER,
//! which is the encoding Fabric and X.509 both expect.

use p256::ecdsa::signature::{Signer, Verifier};
use p256::ecdsa::{Signature, SigningKey, VerifyingKey};

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn from_hex(text: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if !cleaned.len().is_multiple_of(2) {
        return Err(format!(
            "பதினாறு எண்ணிக்கை இரட்டையாக இருக்க வேண்டும்  (hex needs an even number of digits, got {})",
            cleaned.len()
        ));
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16).map_err(|_| {
                format!(
                    "'{}' பதினாறு அல்ல  ('{}' is not hex)",
                    &cleaned[i..i + 2],
                    &cleaned[i..i + 2]
                )
            })
        })
        .collect()
}

/// A new key pair: the private scalar and the public point, both as hex.
///
/// The randomness comes straight from the operating system rather than through
/// a generator threaded from elsewhere: a key is only as good as where its
/// bytes came from, and this is the shortest path from the OS to the key.
///
/// Not every 32-byte string is a valid P-256 scalar — zero is not, and neither
/// is anything at or above the curve order. The odds of drawing one are far
/// below the odds of the hardware being wrong, but "far below" is not "never",
/// so it draws again rather than pretending.
pub fn generate() -> (String, String) {
    let signing = loop {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).expect("the operating system refused randomness");
        if let Ok(signing) = SigningKey::from_slice(&bytes) {
            break signing;
        }
    };
    let verifying = VerifyingKey::from(&signing);
    (
        to_hex(&signing.to_bytes()),
        to_hex(verifying.to_sec1_point(false).as_bytes()),
    )
}

/// The public key belonging to a private one.
///
/// Worth having on its own: a key pair is generated once and the private half
/// stored, and the public half is then wanted every time it is shared.
pub fn public_of(private_hex: &str) -> Result<String, String> {
    let signing = signing_key(private_hex)?;
    Ok(to_hex(
        VerifyingKey::from(&signing).to_sec1_point(false).as_bytes(),
    ))
}

fn signing_key(private_hex: &str) -> Result<SigningKey, String> {
    let bytes = from_hex(private_hex)?;
    SigningKey::from_slice(&bytes).map_err(|_| {
        "தனிச்சாவி சரியில்லை  (not a valid P-256 private key: it must be 32 bytes, and not zero)"
            .to_string()
    })
}

/// Sign a message. The digest is SHA-256, and the signature is DER.
pub fn sign(message: &str, private_hex: &str) -> Result<String, String> {
    let signing = signing_key(private_hex)?;
    let signature: Signature = signing.sign(message.as_bytes());
    Ok(to_hex(signature.to_der().as_bytes()))
}

/// Does this signature belong to this message and this public key?
///
/// Answers false rather than erroring for a signature that simply does not
/// verify — that is an ordinary outcome and a program has to handle it. A key
/// or a signature that is not well formed at all is a different thing, and
/// says so.
/// Sign, as the fixed-width r||s pair rather than DER.
///
/// The same signature over the same message; only the encoding differs. DER is
/// what most things mean by an ECDSA signature, and XML Signature is not one of
/// them: it specifies the IEEE P1363 form, sixty-four bytes with r and s each
/// padded to the curve size. A DER SignatureValue verifies against itself and
/// against nothing else, which is the failure that gets found by a
/// counterparty rather than by a test.
pub fn sign_fixed(message: &str, private_hex: &str) -> Result<Vec<u8>, String> {
    let key_bytes = from_hex(private_hex)?;
    let signing = SigningKey::from_slice(&key_bytes)
        .map_err(|_| "தனிச் சாவி செல்லாதது  (not a valid private key)".to_string())?;
    let signature: Signature = signing.sign(message.as_bytes());
    Ok(signature.to_bytes().to_vec())
}

/// Verify a fixed-width r||s signature.
pub fn verify_fixed(message: &str, signature: &[u8], public_hex: &str) -> Result<bool, String> {
    let key_bytes = from_hex(public_hex)?;
    let verifying = VerifyingKey::from_sec1_bytes(&key_bytes)
        .map_err(|_| "பொதுச் சாவி செல்லாதது  (not a valid public key)".to_string())?;
    let parsed = match Signature::from_slice(signature) {
        Ok(value) => value,
        Err(_) => return Ok(false),
    };
    Ok(verifying.verify(message.as_bytes(), &parsed).is_ok())
}

pub fn verify(message: &str, signature_hex: &str, public_hex: &str) -> Result<bool, String> {
    let key_bytes = from_hex(public_hex)?;
    let verifying = VerifyingKey::from_sec1_bytes(&key_bytes).map_err(|_| {
        "பொதுச்சாவி சரியில்லை  (not a valid P-256 public key: expected an uncompressed SEC1 point)"
            .to_string()
    })?;

    let signature_bytes = from_hex(signature_hex)?;
    let signature = match Signature::from_der(&signature_bytes) {
        Ok(signature) => signature,
        // Malformed DER is not a valid signature for anything, which is the
        // answer the caller asked for.
        Err(_) => return Ok(false),
    };

    Ok(verifying.verify(message.as_bytes(), &signature).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signature_verifies_against_its_own_key() {
        let (private, public) = generate();
        let signature = sign("ஒப்பந்தம் 1000", &private).unwrap();

        assert!(verify("ஒப்பந்தம் 1000", &signature, &public).unwrap());
    }

    #[test]
    fn a_changed_message_does_not_verify() {
        // The whole point: the signature covers the message, so altering the
        // amount after signing has to be detectable.
        let (private, public) = generate();
        let signature = sign("ஒப்பந்தம் 1000", &private).unwrap();

        assert!(!verify("ஒப்பந்தம் 9000", &signature, &public).unwrap());
    }

    #[test]
    fn another_key_does_not_verify() {
        let (private, _) = generate();
        let (_, other_public) = generate();
        let signature = sign("பரிவர்த்தனை", &private).unwrap();

        assert!(!verify("பரிவர்த்தனை", &signature, &other_public).unwrap());
    }

    #[test]
    fn the_public_key_can_be_derived_from_the_private_one() {
        let (private, public) = generate();

        assert_eq!(public_of(&private).unwrap(), public);
    }

    #[test]
    fn rubbish_is_refused_rather_than_answered() {
        let (_, public) = generate();

        assert!(sign("m", "not-hex").is_err(), "a key that is not hex");
        assert!(sign("m", "00").is_err(), "a key of the wrong length");
        assert!(
            verify("m", "00", "not-hex").is_err(),
            "a public key that is not hex"
        );

        // A well-formed request whose signature is simply wrong is false, not
        // an error — a program has to be able to handle that outcome.
        assert!(!verify("m", "3006020100020100", &public).unwrap());
    }

    #[test]
    fn two_signatures_over_one_message_both_verify() {
        // ECDSA is randomised, so signing twice gives two different signatures
        // and both are valid. A test that expected them to match would fail
        // for the wrong reason.
        let (private, public) = generate();
        let first = sign("மாதாந்திர அறிக்கை", &private).unwrap();
        let second = sign("மாதாந்திர அறிக்கை", &private).unwrap();

        assert!(verify("மாதாந்திர அறிக்கை", &first, &public).unwrap());
        assert!(verify("மாதாந்திர அறிக்கை", &second, &public).unwrap());
    }
}
