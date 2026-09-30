# oppuqal — ஒப்புதல்

OAuth 2.0, client side: getting a token, keeping it, and knowing when it has stopped being worth sending.

Three grants, which are the three anything current should use — **authorization code with PKCE**, **refresh**, and **client credentials**. Implicit and resource-owner-password are not here and are not planned: both are withdrawn in OAuth 2.1, and a library that still offers them is a library whose weaker option gets chosen.

## Nothing here reads the clock or invents a random number

`இப்போது_நொடி` is an argument, and the unguessable values come from functions you call by name. So a whole flow replays in a test with fixed inputs and is still the same code path — which matters more here than elsewhere, because a token library whose expiry decisions cannot be tested is one whose expiry decisions are not tested. `oruwkiNYppu` states the same rule for the same reason.

The one function that touches the network, `சீட்டைக்_கோரு`, is kept apart from the ones that build what it sends, so those need no server to exercise.

## What it refuses

- A redirect whose `state` does not match the one sent — that redirect belongs to a request that was not yours, and the check happens *before* the code is spent.
- A redirect carrying `error`, or no code at all.
- A token response with no `access_token`, or one that is not JSON. Returning an empty string here would put an empty `Bearer` header on the next request.
- PKCE's `plain` method. It sends the verifier itself, which is the one thing PKCE exists to avoid sending.

A server that states no `expires_in` gets no expiry invented for it: the honest record of "it did not say" is no expiry, and `காலாவதியானதா` answers `பொய்` rather than guessing.

## Files

| File | What it holds | Functions |
|---|---|---|
| `oppuqal.qmz` | the client side of OAuth 2.0 | `விடுவி` `எழுமாற்றுச்_சரம்` `காவல்_குறியை_ஆக்கு` `சரிபார்ப்பானை_ஆக்கு` `சவால்_ஆக்கு` `வாடிக்கையாளர்_ஆக்கு` `அங்கீகார_முகவரியை_ஆக்கு` `வினவலைப்_படி` `திருப்பத்தைப்_படி` `அனுமதிக்_கோரிக்கை` `புதுப்பிப்புக்_கோரிக்கை` `வாடிக்கையாளர்_சான்றுக்_கோரிக்கை` `சீட்டுத்_தலைப்புகள்` `சீட்டைப்_படி` `காலாவதியானதா` `புதுப்பிக்கவா` `தாங்கித்_தலைப்பு` `சீட்டைக்_கோரு` |
| `oppuqal_cOqaZY.qmz` | 39 tests, including RFC 7636's own PKCE vector | — |

## Two host primitives arrived with this

`சுருக்கம்_256` (SHA-256, as bytes) and `எழுமாற்று` (that many unguessable bytes). Both are Layer 0 by the standard in `nUlakam/README.md` — only what cannot be expressed in the language itself. A digest is bit work and eTamil has no bit operators; and before `எழுமாற்று` the clock was the only source of variety an eTamil program had, which makes every `state` and `nonce` derivable by whoever else can read a clock.

The encoding stayed in eTamil: `../vativam/kuRiyAkkam.qmz` gained `அறுபத்துநான்கு_பைட்டுகளில்` and `அறுபத்துநான்கு_உரலி`, so the host hands back bytes and eTamil turns them into base64url.

## Why `விடுவி` and not UPI's `உரை_மறை`

`upi/vilAcam.qmz` escapes six characters on purpose, so a pay link still reads well in a payer's app. That is right there and wrong here: a `redirect_uri` is full of `:` and `/`, and a bare `+` means a space to a form parser. `விடுவி` escapes everything outside RFC 3986's unreserved set.

## uqavi — உதவி (worked examples)

`uqavi/` holds runnable programs named after the function they demonstrate:
`awkIkAra_mukavariyY_Akku.qmz` starts a flow, `qiruppaqqYp_pati.qmz` is the
`state` check, and `cIttYp_pati.qmz` is expiry. Unlike the older packages this
is not yet one per function — the remaining ones are worth adding.

```bash
bash scripts/run_samples.sh oppuqal
```
