# kaNakkiyal — கணக்கியல்

Double-entry bookkeeping: the chart of accounts, the ledger, and the statements that come out of it.

The ledger refuses an unbalanced transaction rather than posting it and leaving someone to find it later. Everything above it — the trial balance, the income statement, the balance sheet — is derived from postings rather than stored, so the statements cannot disagree with the ledger they came from.

Also here: GST on transactions, the rate that applied on a given date, depreciation, payroll, period close, and multi-company and multi-currency support.

## Files

| File | What it holds | Functions |
|---|---|---|
| `Uqiyam.qmz` | ஊதியம் (payroll) | `மொத்தச்_சம்பளம்` `நாட்களுக்கு_ஏற்ப` `வரம்புடன்_பங்களிப்பு` `தகுதிக்குள்_பங்களிப்பு` `படிநிலை_வரி` `பணிக்கொடை` `சம்பளச்_சீட்டு` |
| `aRikkYkaL.qmz` | அறிக்கைகள் (financial statements) | `இருப்பாய்வு` `வருமான_அறிக்கை` `இருப்புநிலை` `கால_வருமான_அறிக்கை` `நாள்_இருப்புநிலை` `கால_இருப்பாய்வு` `கணக்கு_அறிக்கை` `பணப்புழக்க_அறிக்கை` `இருப்பாய்வு_அச்சிடு` `வருமான_அறிக்கை_அச்சிடு` `இருப்புநிலை_அச்சிடு` |
| `coqqu_Uqiyam_cOqaZY.qmz` | tests for depreciation and payroll | — |
| `kAlam.qmz` | காலம் (reporting periods) | `காலம்_ஆக்கு` `இந்திய_ஆண்டு` `நாட்காட்டி_ஆண்டு` `காலத்தில்_உள்ளதா` `வரையிலா` `காலம்_விவரம்` |
| `kaNakkukaL.qmz` | கணக்குகள் (chart of accounts) | `வகை_சொத்து` `வகை_பொறுப்பு` `வகை_பங்கு` `வகை_வருவாய்` `வகை_செலவு` `கணக்கு_ஆக்கு` `செல்லுபடியா` `பற்று_இயல்பா` `இருப்புநிலைக்கானதா` `கணக்கு_தேடு` `வகையால்_வடிகட்டு` |
| `mutippu.qmz` | ஆண்டு முடிப்பு (period close) | `முடிப்பு_பரிவர்த்தனை` `ஆண்டை_முடி` `முடிக்கப்பட்டதா` |
| `niRuvaZam.qmz` | நிறுவனம் and நாணயம் | `நிறுவனம்_ஆக்கு` `நிறுவனத்துடன்_பதிவிடு` `நிறுவன_வடிகட்டு` `நாணயம்_ஆக்கு` `மாற்று_விகிதம்_ஆக்கு` `அடிப்படைக்கு_மாற்று` `வேறுபாட்டுத்_தொகை` `அன்னிய_வேறுபாடு` |
| `oqukkItu.qmz` | ஒதுக்கீடு (assignment / clearing) | `ஒதுக்கீடு_ஆக்கு` `ஒதுக்கப்பட்ட_மொத்தம்` `பயன்படுத்தப்பட்டது` `நிலுவைத்_தொகை` `ஒதுக்கு` `நிலுவைப்_பட்டியல்` `வயது_அட்டவணை` `வயது_அட்டவணை_அச்சிடு` |
| `pErEtu.qmz` | பேரேடு (the ledger) | `வரிசை_ஆக்கு` `பற்று_வரிசை` `வரவு_வரிசை` `பரிவர்த்தனை_ஆக்கு` `மொத்த_பற்று` `மொத்த_வரவு` `சமநிலையா` `பதிவிடு` `கணக்கு_இருப்பு` `கணக்கு_பதிவுகள்` `காலம்_வடிகட்டு` `வரை_வடிகட்டு` `பரிவர்த்தனை_தொகை` `பரிவர்த்தனை_நாள்` `பரிவர்த்தனை_எண்கள்` |
| `qEymAZam.qmz` | தேய்மானம் (depreciation) | `நேர்கோட்டு_ஆண்டு` `நேர்கோட்டு_விகிதம்` `குறையும்_ஆண்டு` `பகுதி_ஆண்டு` `நேர்கோட்டு_அட்டவணை` `குறையும்_அட்டவணை` `மொத்தத்_தேய்வு` `தொகுதி_தேய்வு` |
| `vari.qmz` | வணிகவரி (GST) on transactions | `வரி_விகிதம்_ஆக்கு` `வரி_தொகை` `வரியுடன்_மொத்தம்` `அடிப்படையை_பிரி` `மாநில_பிரிப்பு` `விற்பனை_பரிவர்த்தனை` `கொள்முதல்_பரிவர்த்தனை` `பணம்_பெறு` `பணம்_செலுத்து` `ரொக்க_விற்பனை` `ரொக்க_கொள்முதல்` `வரவு_குறிப்பு` `பற்று_குறிப்பு` `எதிர்_பதிவு` `தொடக்க_இருப்பு` |
| `vari_cOqaZY.qmz` | tests for the GST module | — |
| `vari_vikiqam.qmz` | finding the rate that applied | `விகிதம்_தேடு` `படிகளை_ஏற்று` `படி_வரி_கணக்கிடு` `உள்_மாநிலமா` `மாநிலப்_பெயர்` `மாநிலங்கள்` `சரிபார்க்கப்படாத_விகிதங்கள்` |
| `vari_vikiqam_cOqaZY.qmz` | tests for finding the rate that applied | — |

`vari_vikiqam.qmz` answers "what rate applied *then*", which is a different question from "what rate applies now" and the one an amended return actually needs.

## uqavi — உதவி (worked examples)

`uqavi/` holds 99 runnable programs, one for each function above, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh kaNakkiyal
```
