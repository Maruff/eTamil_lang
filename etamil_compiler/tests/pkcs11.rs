//! PKCS#11 against a real token, when there is one.
//!
//! `#[ignore]`, and run with `-- --ignored` when a token is there:
//!
//!     cargo test --features pkcs11 --test pkcs11 -- --ignored
//!
//! Not an `if env::var(...) { return }` gate, which is how the MySQL example is
//! opted into. The difference matters here: a test that returns early is
//! counted as passing, and this is the only place the FFI is exercised at all —
//! so a machine with no token would report five green tests over code that never
//! ran. Ignored tests are reported as ignored.
//!
//! CI runs them on Ubuntu against SoftHSM2, which `apt-get` has.
//! `docs/backend/PKCS11.md` has the four commands to set one up yourself.
//!
//! The test that matters is the last one. A signature the device produces has to
//! verify with `p256` — the same code `வளைவு_நேர்_சரிபார்` uses — because that
//! is the claim being made: that a key in an HSM and a key in a hex string are
//! interchangeable everywhere downstream, including in an XML signature.

#![cfg(all(feature = "pkcs11", not(target_family = "wasm")))]

use etamil_compiler::pkcs11;

fn setting(name: &str, fallback: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| fallback.to_string())
}

fn session() -> i64 {
    let library = setting("ETAMIL_PKCS11_LIB", "/usr/lib/softhsm/libsofthsm2.so");
    let token = setting("ETAMIL_PKCS11_TOKEN", "etamil");
    let pin = setting("ETAMIL_PKCS11_PIN", "1234");

    pkcs11::open(&library, &token, &pin).unwrap_or_else(|why| {
        panic!(
            "no token opened: {why}
{HOW}"
        )
    })
}

const HOW: &str = "set ETAMIL_PKCS11_LIB, _TOKEN, _PIN and _KEY, or see docs/backend/PKCS11.md";

fn key_label() -> String {
    setting("ETAMIL_PKCS11_KEY", "cIttu")
}

#[test]
#[ignore = "needs a PKCS#11 token: run with -- --ignored"]
fn the_token_lists_the_key_it_was_given() {
    let handle = session();
    let labels = pkcs11::keys(handle).expect("listing keys");
    assert!(
        labels.contains(&key_label()),
        "the token holds {labels:?}, which does not include {}",
        key_label()
    );
    pkcs11::close(handle).expect("closing");
}

#[test]
#[ignore = "needs a PKCS#11 token: run with -- --ignored"]
fn the_public_key_is_a_sec1_point() {
    let handle = session();
    let hex = pkcs11::public_key(handle, &key_label()).expect("reading the public key");

    // 65 bytes, uncompressed, is what `வளைவு_பொதுச்சாவி` produces and what
    // `வளைவு_நேர்_சரிபார்` takes. Anything else here would be a string that
    // looks like a key and verifies nothing.
    assert_eq!(hex.len(), 130, "expected 65 bytes of SEC1 point, got {hex}");
    assert!(hex.starts_with("04"), "not an uncompressed point: {hex}");
    pkcs11::close(handle).expect("closing");
}

#[test]
#[ignore = "needs a PKCS#11 token: run with -- --ignored"]
fn a_signature_the_device_made_verifies_here() {
    let handle = session();
    let label = key_label();
    let message = "<SignedInfo>the bytes an XML signature covers</SignedInfo>";

    let signature = pkcs11::sign(handle, &label, message).expect("signing");
    assert_eq!(
        signature.len(),
        64,
        "CKM_ECDSA gives r||s; {} bytes is DER or something else",
        signature.len()
    );

    let public = pkcs11::public_key(handle, &label).expect("reading the public key");
    assert!(
        etamil_compiler::signing::verify_fixed(message, &signature, &public).unwrap(),
        "the device signed it and this build does not accept it"
    );

    // And it is a signature over *that* message, not one that verifies for
    // anything — which a test that only checked the happy path would not tell
    // apart.
    assert!(
        !etamil_compiler::signing::verify_fixed("something else", &signature, &public).unwrap()
    );
    pkcs11::close(handle).expect("closing");
}

#[test]
#[ignore = "needs a PKCS#11 token: run with -- --ignored"]
fn a_closed_session_is_not_a_session() {
    let handle = session();
    pkcs11::close(handle).expect("closing");
    assert!(pkcs11::keys(handle).is_err());
    assert!(pkcs11::close(handle).is_err());
}

#[test]
#[ignore = "needs a PKCS#11 token: run with -- --ignored"]
fn a_key_the_token_does_not_hold_is_refused() {
    let handle = session();
    let error = pkcs11::sign(handle, "no-such-key", "x").unwrap_err();
    assert!(error.contains("no key is labelled that"), "{error}");
    pkcs11::close(handle).expect("closing");
}
