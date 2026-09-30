# muqqirY — முத்திரை

XML Signature: signing a document from inside it, and checking one that arrives.

This is the envelope — `Reference`, `DigestValue`, `SignedInfo`, `SignatureValue` — over the canonicalization `நேர்வடிவம்` does. It signs with ECDSA over P-256 and digests with SHA-256, which is what an Aadhaar authentication response, a GSTN acknowledgement and a SAML assertion all arrive carrying.

## The transform, which is the part worth understanding

A signature that sits inside the document it signs cannot be part of what it signs. So before the document is digested, the `Signature` element comes out — and it has to come out *exactly*, leaving every other byte where it was.

That is why `உறுப்பைத்_தேடு` hands back three things, and why `மூலம்` is one of them: the element's own bytes, straight out of the source. Cutting that substring out restores the document to what was digested. Re-serializing the rest instead would tidy indentation that was there when it was signed, and produce a different digest from the same document.

`முத்திரையை_நீக்கு` is that one substitution, and it is the whole of the enveloped-signature transform. `முத்திரையைச்_சேர்` is its inverse: the signature goes in as the last child of the root with nothing added around it, so removing it again gives back the original byte for byte. There is a test that says exactly that, because everything else rests on it.

## Two questions, in this order

`முத்திரையைச்_சரிபார்` asks:

1. **Was `SignedInfo` signed by the holder of this key?** Until that answers yes, the `DigestValue` inside `SignedInfo` is a number an attacker chose, and comparing the document against it proves nothing.
2. **Does the document still digest to that value?**

The order is not a preference. Reversing it makes a document that was altered *and* had its `DigestValue` corrected to match look valid until the second check — and a verifier that reported the first failure it found would then report the wrong one. There is a test that alters a document, corrects the digest, and checks the refusal says *the key*, not *the document*.

`சரி(மெய்)` is the only success. Each failure says which of the two gave way, because "altered after signing" and "not signed by this key" call for different actions.

## What it refuses

- **Signing a document that already carries a signature.** The second signature would digest the first one away, and the result would be indistinguishable from a replacement. There is no reading of that which is not a guess.
- **An `Id` no element carries** — rather than signing the whole document instead, which would verify and cover something else.
- **An empty root element**, `<Resp/>`. An enveloped signature has to go inside the root, and that one has no inside.
- **A `Reference` pointing outside the document.** Nothing is fetched to check it. A verifier that went to the network to find what it was checking is one whose target an attacker picks.
- **Verifying an unsigned document** answers `தவறு`, not `பொய்` — "there is no signature here" and "the signature failed" are different facts, and the second is what `பொய்` would be read as.

## What it does not do

No `KeyInfo`, so the public key is an argument rather than something read out of the document. That is deliberate in the same way the refusals above are: a verifier that takes the key from the document it is checking verifies that the document is internally consistent and nothing else. Which key to trust is a question about certificates and trust stores, and it belongs to the caller.

One `Reference` per signature. Multiple references are what a detached signature over several files uses, and nothing here is detached.

Inclusive canonicalization is available in `நேர்வடிவம்` but is not what this writes. Exclusive is what lets a signed element be quoted inside a larger message later without the signature breaking, which is the case that arises.

## Files

| File | What it holds | Functions |
|---|---|---|
| `muqqirY.qmz` | XML Signature, enveloped | `சுருக்கக்_குறிப்பு` `முத்திரையை_நீக்கு` `குறிப்பை_ஆக்கு` `கையொப்பத்_தகவலை_ஆக்கு` `முத்திரையைச்_சேர்` `கையொப்பமிட_வேண்டியது` `முத்திரையை_முடி` `முத்திரையிடு` `வன்சாவியால்_முத்திரையிடு` `முத்திரையைச்_சரிபார்` |
| `muqqirY_cOqaZY.qmz` | 42 tests | — |
| `uqavi/muqqirYyitu.qmz` | sign a document, then check it | — |
| `uqavi/uRYpoqi_mARRam.qmz` | the enveloped transform on its own | — |
| `uqavi/AqAr_paqil.qmz` | an Aadhaar-shaped response, signed by `Id` | — |

## Signing with a key that is not here

`முத்திரையிடு` takes a private key in hexadecimal, which means the key is in this process — and for anything that matters it should not be. So the three steps are separable:

| Step | Knows the key? |
|---|---|
| `கையொப்பமிட_வேண்டியது(ஆவணம், அடையாளம்)` | no — returns the SignedInfo and the canonical bytes to sign |
| a signature over those bytes | yes, and only this one |
| `முத்திரையை_முடி(ஆவணம், வேண்டியது, பைட்டுகள்)` | no — assembles the signed document |

`முத்திரையிடு` is those three with `வளைவு_நேர்_கையொப்பம்` in the middle. `வன்சாவியால்_முத்திரையிடு` is the same three with `வன்சாவி_கையொப்பம்`, and the private key never leaves the HSM. The two differ in one line, which is what moving a signing key into a device ought to cost.

That works because `CKM_ECDSA` returns r and s side by side — the same encoding XML Signature specifies — so a signature from a device and a signature from a hex string are indistinguishable downstream. Verification is the ordinary `முத்திரையைச்_சரிபார்` either way, with no device present: a counterparty checking a signed invoice has no token and needs none. `docs/backend/PKCS11.md` has the rest, including why `CKM_ECDSA` and not `CKM_ECDSA_SHA256`.

There is a test for the seam itself: doing the three steps by hand must produce the same SignedInfo as `முத்திரையிடு` does, or `வன்சாவியால்_முத்திரையிடு` is a second implementation wearing the same name.

## Verified against something else

A signature that verifies against itself and against nothing else is the failure this library is most likely to have, and the one its own tests cannot find — both sides would be wrong in the same way and agree. So the output was checked by `lxml` and `cryptography` instead, which share no code with this: four documents — flat, indented with an `Id` reference and reordered attributes, prefixed namespaces with a declaration nothing uses, and empty elements — canonicalized by `lxml`, digested by `hashlib`, and the ECDSA checked by `cryptography` against the `r||s` bytes. All four agreed.

The one test here that guards the same property from the inside is the last: `SignedInfo` canonicalized on its own must be the same bytes as `SignedInfo` read back out of the finished `Signature`. If those ever diverge, every signature this produces verifies nowhere else, and every other test still passes.

## Nothing here reads the clock

The private key is an argument. `வளைவு_நேர்_கையொப்பம்` is the one call whose output varies between runs — ECDSA picks a fresh nonce — so the tests either sign and verify in one breath, or pin the digest, which is deterministic. The key the tests use is RFC 6979's own P-256 example key: a published value rather than one invented here. `oruwkiNYppu` states the rule and the reason.

## uqavi — உதவி (worked examples)

`uqavi/` holds runnable programs named after what they show: `muqqirYyitu.qmz`
signs and checks, `uRYpoqi_mARRam.qmz` is the enveloped transform with nothing
else happening around it, and `AqAr_paqil.qmz` is an Aadhaar-shaped response
signed by `Id`.

```bash
bash scripts/run_samples.sh muqqirY
```
