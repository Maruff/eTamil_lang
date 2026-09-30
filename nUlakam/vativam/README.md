# vativam — வடிவம்

Formats — the shapes data takes when it leaves the program.

JSON, base64 and hex, and a document renderer that fills a template inside an .odt, .ods or .docx package.

The base64 functions come in pairs: one that speaks text and one that speaks bytes. That is not symmetry for its own sake — a SHA-256 digest is thirty-two arbitrary bytes and an ECDSA signature sixty-four, and almost none of those byte strings are valid UTF-8, so a function that ends in `பைட்டுச்_சரம்` has to refuse them. `அறுபத்துநான்கு_பைட்டுகளில்` and `அறுபத்துநான்கு_பைட்டுகளாக` are the two halves that do not.

`jEcAZ.qmz` is mostly gone: its parser and writer are host builtins now, because a recursive descent parser written in eTamil paid the language's string and record costs on every character and could not finish a 624 KB document. What is left is `ஜேசான்_சரம்`, which quotes one string and is not on any hot path.

## The PDF path shapes, and that is now checked

`_pdf_ஆக்கு` converts through LibreOffice, which shapes with HarfBuzz. That has always been the answer to "does eTamil produce correct Tamil PDFs", and until now it was an answer nobody had tested — the kind of claim that stays true until a font or a version changes and then stays *stated* for a year, because a PDF with wrongly-ordered Tamil still opens, still prints, and still extracts back to the right characters.

`scripts/check_pdf_shaping.py` reads the glyphs out of the page's content stream instead of the text, and asks two questions that are properties of the writing systems rather than of any font:

| Written | Glyphs | What it shows |
|---|---|---|
| `க` | `01` | — |
| `கெ` | `02 01` | the vowel sign is drawn *before* the consonant |
| `கொ` | `02 01 03` | ொ split in two and the consonant went between them |
| `س` | `01` | — |
| `سسس` | `02 03 04` | one letter, three positions, three glyphs |

Unshaped output would put க first in `கொ` and draw `سسس` as `01 01 01`. The script's `--self-test` runs both of those past the checks to confirm they are caught, because a check nothing has ever failed is a check that has not been shown to be able to fail. CI runs it on every push.

A native shaper was considered and is not being written. HarfBuzz is what every correct implementation uses, LibreOffice already calls it, and a second one here would be weeks of work to arrive somewhere worse.

## Files

| File | What it holds | Functions |
|---|---|---|
| `AvaNam.qmz` | render a document template | `ஆவணத்தில்_உள்ளதா` `_xml_ஆக்கு` `பகுதியை_எடு` `வரிசைக்குப்_பின்` `மாறிப்_பெயர்` `உறுப்பை_நிரப்பு` `தொகுதியைத்_தேடு` `ஆவணம்_நிரப்பு` `பிளந்த_குறிகளை_இணை` `பொதியை_நிரப்பு` `_pdf_ஆக்கு` |
| `jEcAZ.qmz` | ஜேசான் (JSON), written in eTamil | `எழுத்து_மறை` `ஜேசான்_சரம்` |
| `kuRiyAkkam.qmz` | குறியாக்கம் (encoding): base64 and hex | `அறுபத்துநான்கு_எழுத்துகள்` `அறுபத்துநான்கு_உரலி_எழுத்துகள்` `பதினாறு_எழுத்துகள்` `அறுபத்துநான்கு_ஆக்கு` `அறுபத்துநான்கு_பைட்டுகளில்` `அறுபத்துநான்கு_உரலி` `அறுபத்துநான்கு_படி` `அறுபத்துநான்கு_பைட்டுகளாக` `பதினாறு_ஆக்கு` `பதினாறு_படி` |

## uqavi — உதவி (worked examples)

`uqavi/` holds 20 runnable programs, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh vativam
```
