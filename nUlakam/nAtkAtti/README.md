# nAtkAtti — நாட்காட்டி

Dates and working days.

`nAL.qmz` is the calendar itself: validity, leap years, month ends, adding months and years, quarters and financial years. It checks the shape of a date without asking the host, so the same code runs on a board with no clock.

`vElYnAL.qmz` adds a calendar of weekly offs and holidays, so "five working days from now" is a question that can be answered.

## Files

| File | What it holds | Functions |
|---|---|---|
| `nAL.qmz` | நாள், the calendar itself | `காலத்_தொடக்கம்` `நாள்_வடிவமா` `நாள்_செல்லுபடியா` `நாள்_சரிபார்` `நாளாக_வடிவமை` `நாள்_ஆக்கு` `ஆண்டைப்_பெறு` `மாதத்தைப்_பெறு` `நாளைப்_பெறு` `நெட்டாண்டா` `மாத_நாட்கள்` `மாத_முதல்` `மாத_இறுதி` `மாத_இறுதியா` `மாதங்களைக்_கூட்டு` `ஆண்டுகளைக்_கூட்டு` `முடிந்த_மாதங்கள்` `முடிந்த_ஆண்டுகள்` `பகுதி_மாதங்கள்` `முந்தையது` `பிந்தையது` `வாரநாள்` `வாரநாள்_பெயர்` `நிதியாண்டின்_தொடக்க_ஆண்டு` `கால்_ஆண்டு_எண்` |
| `nAtkAtti_cOqaZY.qmz` | tests for the calendar | — |
| `vElYnAL.qmz` | வேலைநாள் (working days) | `தேடல்_எல்லை` `நாட்காட்டி_ஆக்கு` `பட்டியலில்_உள்ளதா` `வார_ஓய்வா` `விடுமுறையா` `வேலை_நாளா` `வேலை_வாரம்_உள்ளதா` `அடுத்த_வேலை_நாள்` `முந்தைய_வேலை_நாள்` `நகர்த்தி_வேலை_நாள்` `வேலை_நாட்களைக்_கூட்டு` `வேலை_நாட்களை_எண்ணு` `முடிவு_நாள்` `பணி_அட்டவணை` `உருட்டு` |
