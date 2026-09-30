//! PKCS#11: signing with a key that is not in the process.
//!
//! An HSM's whole proposition is that the private key never leaves it. So the
//! shape here is not "fetch a key and sign" — it is "hand the device something
//! to sign and take back the signature". There is no function that returns a
//! private key, and there is no place one could be put if there were.
//!
//! `cryptoki` opens the vendor's library at runtime rather than linking it, so
//! this builds on a machine with no HSM and no vendor SDK. Only the tests need
//! a device, and `tests/pkcs11.rs` runs against SoftHSM2 when
//! `ETAMIL_TEST_PKCS11=1` says one is there — the same shape as
//! `ETAMIL_TEST_MYSQL`.
//!
//! ## What is signed
//!
//! SHA-256 of the message, then `CKM_ECDSA` over the digest. Not
//! `CKM_ECDSA_SHA256`, which hashes inside the device: that mechanism is
//! optional in PKCS#11 and a token that lacks it would fail at signing time
//! rather than here. Every token that does ECDSA at all does `CKM_ECDSA`.
//!
//! `CKM_ECDSA` returns r and s side by side, each padded to the curve size —
//! IEEE P1363, which is what XML Signature specifies and what
//! `வளைவு_நேர்_சரிபார்` verifies. So a signature made here and a signature made
//! by `வளைவு_நேர்_கையொப்பம்` are the same shape, and `muqqirY` takes either.
//!
//! ## Sessions belong to the thread that opened them
//!
//! `cryptoki::Session` is deliberately neither `Send` nor `Sync`, so an open
//! session lives in a thread-local table and its handle is meaningless on
//! another thread. For a program that signs, that is the whole story. For a
//! server, open a session inside the handler rather than once at startup —
//! `வன்சாவி_திற` from one thread and `வன்சாவி_கையொப்பம்` from another answers
//! "no such session" rather than signing with the wrong key, which is the
//! failure worth having.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
use cryptoki::mechanism::Mechanism;
use cryptoki::object::{Attribute, AttributeType, ObjectClass, ObjectHandle};
use cryptoki::session::{Session, UserType};
use cryptoki::types::AuthPin;

thread_local! {
    static OPEN: RefCell<HashMap<i64, Session>> = RefCell::new(HashMap::new());
    static NEXT: Cell<i64> = const { Cell::new(1) };
}

fn why(context: &str, error: impl std::fmt::Display) -> String {
    format!(
        "வன்சாவி: {} — {}  (HSM: {}: {})",
        context, error, context, error
    )
}

/// Open a session on a token and log in.
///
/// `token` is the token's label, not a slot number: a slot number changes when
/// a device is re-plugged or another one is added, and a program that signs
/// with whatever is in slot 0 signs with whatever is in slot 0. An empty label
/// takes the first slot that has a token, which is what a single-token test
/// setup wants and what a production one should not rely on.
pub fn open(library: &str, token: &str, pin: &str) -> Result<i64, String> {
    let context = Pkcs11::new(library)
        .map_err(|error| why(&format!("'{}' திறக்க முடியவில்லை", library), error))?;
    context
        .initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK))
        .map_err(|error| why("தொடக்கம்", error))?;

    let slots = context
        .get_slots_with_token()
        .map_err(|error| why("இடங்கள்", error))?;

    let slot = if token.is_empty() {
        slots
            .first()
            .copied()
            .ok_or_else(|| "வன்சாவி: எந்த இடத்திலும் டோக்கன் இல்லை  (no slot has a token)".to_string())?
    } else {
        let mut found = None;
        for slot in &slots {
            if let Ok(info) = context.get_token_info(*slot)
                && info.label().trim() == token
            {
                found = Some(*slot);
                break;
            }
        }
        found.ok_or_else(|| {
            format!(
                "வன்சாவி: '{}' என்ற டோக்கன் இல்லை  (no token is labelled that)",
                token
            )
        })?
    };

    let session = context
        .open_rw_session(slot)
        .map_err(|error| why("அமர்வு", error))?;
    session
        .login(UserType::User, Some(&AuthPin::new(pin.into())))
        .map_err(|error| why("உள்நுழைவு", error))?;

    // The context is dropped here on purpose: cryptoki's Session holds its own
    // clone of it, so the library stays loaded for exactly as long as a session
    // is open and no longer.
    let handle = NEXT.with(|next| {
        let value = next.get();
        next.set(value + 1);
        value
    });
    OPEN.with(|open| {
        let _ = open.borrow_mut().insert(handle, session);
    });
    Ok(handle)
}

/// Close a session. Logging out and closing is what returns the token to a
/// state where the next login is a login rather than a no-op.
pub fn close(handle: i64) -> Result<(), String> {
    OPEN.with(|open| match open.borrow_mut().remove(&handle) {
        Some(session) => {
            let _ = session.logout();
            Ok(())
        }
        None => Err(missing(handle)),
    })
}

fn missing(handle: i64) -> String {
    format!(
        "வன்சாவி: {} என்ற அமர்வு இல்லை  (no session {} on this thread)",
        handle, handle
    )
}

fn with_session<T>(
    handle: i64,
    body: impl FnOnce(&Session) -> Result<T, String>,
) -> Result<T, String> {
    OPEN.with(|open| {
        let sessions = open.borrow();
        match sessions.get(&handle) {
            Some(session) => body(session),
            None => Err(missing(handle)),
        }
    })
}

/// The labels of the private keys this session can sign with.
///
/// Private keys, not every object: what a caller can do here is sign, and a
/// list that included certificates and public keys would be a list of names
/// most of which are not answers to the only question being asked.
pub fn keys(handle: i64) -> Result<Vec<String>, String> {
    with_session(handle, |session| {
        let objects = session
            .find_objects(&[Attribute::Class(ObjectClass::PRIVATE_KEY)])
            .map_err(|error| why("சாவிகளைத் தேடுதல்", error))?;

        let mut labels = Vec::with_capacity(objects.len());
        for object in objects {
            if let Ok(values) = session.get_attributes(object, &[AttributeType::Label]) {
                for value in values {
                    if let Attribute::Label(bytes) = value {
                        labels.push(String::from_utf8_lossy(&bytes).trim().to_string());
                    }
                }
            }
        }
        Ok(labels)
    })
}

fn find_key(session: &Session, label: &str, class: ObjectClass) -> Result<ObjectHandle, String> {
    let found = session
        .find_objects(&[
            Attribute::Class(class),
            Attribute::Label(label.as_bytes().to_vec()),
        ])
        .map_err(|error| why("சாவியைத் தேடுதல்", error))?;

    found
        .into_iter()
        .next()
        .ok_or_else(|| format!("வன்சாவி: '{}' என்ற சாவி இல்லை  (no key is labelled that)", label))
}

/// The public half, as the uncompressed SEC1 point in hexadecimal — the same
/// text `வளைவு_பொதுச்சாவி` produces, so `வளைவு_நேர்_சரிபார்` takes it unchanged.
///
/// `CKA_EC_POINT` is DER: an OCTET STRING wrapping the point. Handing that back
/// raw would be handing back something that looks like a key and verifies
/// nothing, so the wrapper comes off here.
pub fn public_key(handle: i64, label: &str) -> Result<String, String> {
    with_session(handle, |session| {
        let key = find_key(session, label, ObjectClass::PUBLIC_KEY)?;
        let values = session
            .get_attributes(key, &[AttributeType::EcPoint])
            .map_err(|error| why("பொதுச்சாவி", error))?;

        for value in values {
            if let Attribute::EcPoint(der) = value {
                return unwrap_ec_point(&der)
                    .map(|point| point.iter().map(|byte| format!("{:02x}", byte)).collect());
            }
        }
        Err(format!(
            "வன்சாவி: '{}' ஒரு வளைவுச் சாவி அல்ல  ('{}' has no EC point, so it is not an elliptic curve key)",
            label, label
        ))
    })
}

/// Take the point out of the DER OCTET STRING `CKA_EC_POINT` is wrapped in.
///
/// Tolerant of a token that hands back the bare point — some do, against the
/// specification — because the two are told apart by their first byte and
/// guessing is not involved: `0x04` as an OCTET STRING tag is followed by a
/// length, and `0x04` as an uncompressed point marker is followed by 64 bytes
/// of coordinates.
fn unwrap_ec_point(der: &[u8]) -> Result<Vec<u8>, String> {
    if der.len() == 65 && der[0] == 0x04 {
        return Ok(der.to_vec());
    }
    if der.len() < 2 || der[0] != 0x04 {
        return Err(format!(
            "வன்சாவி: பொதுச்சாவியின் வடிவம் தெரியவில்லை ({} பைட்டுகள்)  (the EC point is not DER and not a bare point)",
            der.len()
        ));
    }
    let (length, start) = if der[1] < 0x80 {
        (der[1] as usize, 2)
    } else if der[1] == 0x81 && der.len() > 2 {
        (der[2] as usize, 3)
    } else {
        return Err(
            "வன்சாவி: பொதுச்சாவியின் நீளம் மிக நீளமானது  (the EC point length is longer than any curve needs)"
                .to_string(),
        );
    };
    der.get(start..start + length)
        .map(<[u8]>::to_vec)
        .ok_or_else(|| {
            "வன்சாவி: பொதுச்சாவி துண்டிக்கப்பட்டது  (the EC point is shorter than its own length says)"
                .to_string()
        })
}

/// Sign a message with the named private key. The key stays in the device.
///
/// The digest is computed here and `CKM_ECDSA` signs it, so what comes back is
/// r and s side by side — the form XML Signature specifies and
/// `வளைவு_நேர்_சரிபார்` verifies.
pub fn sign(handle: i64, label: &str, message: &str) -> Result<Vec<u8>, String> {
    with_session(handle, |session| {
        let key = find_key(session, label, ObjectClass::PRIVATE_KEY)?;
        let digest = crate::crypt::sha256(message);
        session
            .sign(&Mechanism::Ecdsa, key, &digest)
            .map_err(|error| why("கையொப்பம்", error))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The DER wrapper comes off, and a bare point is left alone. Both are
    /// reachable without a device, and getting this wrong produces a key that
    /// looks right and verifies nothing.
    #[test]
    fn an_ec_point_comes_out_of_its_der_wrapper() {
        let mut point = vec![0x04];
        point.extend(std::iter::repeat_n(0xab, 64));

        let mut der = vec![0x04, 65];
        der.extend_from_slice(&point);
        assert_eq!(unwrap_ec_point(&der).unwrap(), point);

        let mut long_form = vec![0x04, 0x81, 65];
        long_form.extend_from_slice(&point);
        assert_eq!(unwrap_ec_point(&long_form).unwrap(), point);

        assert_eq!(unwrap_ec_point(&point).unwrap(), point);
    }

    #[test]
    fn a_truncated_ec_point_is_refused() {
        assert!(unwrap_ec_point(&[0x04, 65, 0x04, 0x01]).is_err());
        assert!(unwrap_ec_point(&[]).is_err());
        assert!(unwrap_ec_point(&[0x30, 0x02, 0x01, 0x01]).is_err());
    }

    /// A handle that was never opened, or was opened on another thread, is not
    /// a session. Signing with it has to say so rather than reach for whatever
    /// else is in the table.
    #[test]
    fn a_handle_that_is_not_a_session_says_so() {
        let error = keys(9999).unwrap_err();
        assert!(error.contains("no session"), "{}", error);
    }
}
