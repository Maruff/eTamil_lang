# PKCS#11 — signing with a key that is not in the process

An HSM's whole proposition is that the private key never leaves it. Everything about this interface follows from that one sentence: there is no function here that returns a private key, and there is nowhere to put one if there were. You hand the device something to sign and take back the signature.

Behind `--features pkcs11`. Without it these five names still exist and say so, rather than reporting "no such function" and sending you hunting for a typo in a name that is spelled correctly.

```bash
cargo build --release --features pkcs11
```

`cryptoki` opens the vendor's library at runtime rather than linking it, so the build needs no HSM and no vendor SDK. Only running against a device needs a device.

## The five

| Function | What it does |
|---|---|
| `வன்சாவி_திற(நூலகம், டோக்கன், முள்)` | open a session on a token and log in → `சரி(அமர்வு)` |
| `வன்சாவி_மூடு(அமர்வு)` | log out and close |
| `வன்சாவி_சாவிகள்(அமர்வு)` | the labels this session can sign with |
| `வன்சாவி_பொதுச்சாவி(அமர்வு, பெயர்)` | the public half, as SEC1 hexadecimal |
| `வன்சாவி_கையொப்பம்(அமர்வு, பெயர், செய்தி)` | sixty-four bytes: r and s |

## Why the signature is interchangeable

`வன்சாவி_கையொப்பம்` computes SHA-256 of the message and signs the digest with `CKM_ECDSA`. Not `CKM_ECDSA_SHA256`, which hashes inside the device: that mechanism is optional in PKCS#11, and a token that lacks it would fail at signing time rather than at build time. Every token that does ECDSA at all does `CKM_ECDSA`.

`CKM_ECDSA` returns r and s side by side, each padded to the curve size. That is IEEE P1363 — the encoding XML Signature specifies, and the one `வளைவு_நேர்_கையொப்பம்` produces and `வளைவு_நேர்_சரிபார்` accepts. So the two signing paths are interchangeable:

```etamil
// in this process
முடிவு = முத்திரையிடு(ஆவணம், "", தனிச்சாவி);

// in the device
முடிவு = வன்சாவியால்_முத்திரையிடு(ஆவணம், "", அமர்வு, "cIttu");
```

and verification is the same call either way, with no device present:

```etamil
பொதுச்சாவி = மதிப்பு(வன்சாவி_பொதுச்சாவி(அமர்வு, "cIttu"));
முத்திரையைச்_சரிபார்(முத்திரையிட்டது, பொதுச்சாவி);
```

A counterparty verifying a signed invoice has no token and needs none. That is the property `nUlakam/muqqirY/` was split for: `கையொப்பமிட_வேண்டியது` prepares the bytes, something signs them, `முத்திரையை_முடி` assembles the document — and only the middle step knows where the key lives.

## The token is named, not numbered

`வன்சாவி_திற` takes a token *label*. A slot number changes when a device is re-plugged or another is added, and a program that signs with whatever is in slot 0 signs with whatever is in slot 0. An empty label takes the first slot with a token, which is what a single-token test setup wants and what a production one should not rely on.

## Sessions belong to the thread that opened them

`cryptoki::Session` is deliberately neither `Send` nor `Sync`, so an open session lives in a thread-local table and its handle means nothing on another thread. For a program that signs, that is the whole story. For a server, open the session inside the handler rather than once at startup — `வன்சாவி_திற` on one thread and `வன்சாவி_கையொப்பம்` on another answers "no session" rather than signing with the wrong key, which is the failure worth having.

## A token to test against

SoftHSM2 is a PKCS#11 token in a file. It is **not** an HSM — nothing about it is tamper-resistant and the key is on disk — but it speaks the same interface, and the interface is what the tests are about.

```bash
sudo apt-get install -y softhsm2 opensc
softhsm2-util --init-token --free --label etamil --pin 1234 --so-pin 1234
pkcs11-tool --module /usr/lib/softhsm/libsofthsm2.so \
  --login --pin 1234 --keypairgen --key-type EC:prime256v1 --label cIttu
cargo test --features pkcs11 --test pkcs11 -- --ignored
```

`-- --ignored` because those tests are `#[ignore]` rather than gated on an environment variable. The difference matters: a test that returns early when a variable is unset is *counted as passing*, and this is the only place the FFI is exercised at all — so a machine with no token would report five green tests over code that never ran. Ignored tests are reported as ignored.

CI runs the same four commands on Ubuntu on every push, so the FFI is exercised against a real token whether or not anyone has one locally.

Then the end-to-end example, which signs an invoice and verifies it:

```bash
etamil --vm examples/crypto_samples/vaZcAvi_muqqirY.qmz
```

Point it at a real device by changing the library path and the labels at the top of that file, or by setting `ETAMIL_PKCS11_LIB`, `ETAMIL_PKCS11_TOKEN`, `ETAMIL_PKCS11_PIN` and `ETAMIL_PKCS11_KEY` for the Rust tests.

## What is not here

**No key generation, no import, no deletion.** Those are operator actions with a key ceremony around them, done with the vendor's tools and logged. A language runtime that could create keys in a production HSM is one whose bugs can.

**No RSA.** P-256 is what XML Signature, UPI and every recent Indian specification use, and adding a second algorithm before anything needs it would be adding a second untested path.

**No certificate reading.** Which key to trust is a question about certificates and trust stores, and it belongs above this layer — `முத்திரையைச்_சரிபார்` takes the public key as an argument for the same reason.
