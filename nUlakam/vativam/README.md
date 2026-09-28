# vativam — வடிவம்

Formats — the shapes data takes when it leaves the program.

JSON, base64 and hex, and a document renderer that fills a template inside an .odt, .ods or .docx package.

`jEcAZ.qmz` is mostly gone: its parser and writer are host builtins now, because a recursive descent parser written in eTamil paid the language's string and record costs on every character and could not finish a 624 KB document. What is left is `ஜேசான்_சரம்`, which quotes one string and is not on any hot path.

## Files

| File | What it holds | Functions |
|---|---|---|
| `AvaNam.qmz` | render a document template | `ஆவணத்தில்_உள்ளதா` `_xml_ஆக்கு` `பகுதியை_எடு` `வரிசைக்குப்_பின்` `மாறிப்_பெயர்` `உறுப்பை_நிரப்பு` `தொகுதியைத்_தேடு` `ஆவணம்_நிரப்பு` `பொதியை_நிரப்பு` `_pdf_ஆக்கு` |
| `jEcAZ.qmz` | ஜேசான் (JSON), written in eTamil | `எழுத்து_மறை` `ஜேசான்_சரம்` |
| `kuRiyAkkam.qmz` | குறியாக்கம் (encoding): base64 and hex | `அறுபத்துநான்கு_எழுத்துகள்` `பதினாறு_எழுத்துகள்` `அறுபத்துநான்கு_ஆக்கு` `அறுபத்துநான்கு_படி` `பதினாறு_ஆக்கு` `பதினாறு_படி` |

## uqavi — உதவி (worked examples)

`uqavi/` holds 18 runnable programs, one for each function above, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh vativam
```
