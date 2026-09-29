# nErativari — நேரடிவரி

Direct tax: income, head by head, to the tax finally payable.

Income under each head with the set-off rules between them; Chapter VI-A deductions with their individual and combined ceilings; the slab computation with surcharge, marginal relief and cess; tax deducted at source; advance tax and the interest for paying it late; and deferred tax.

`varikkaNakku.qmz` can compare regimes, because which one costs less is a question about a particular person's income rather than a rule.

## Files

| File | What it holds | Functions |
|---|---|---|
| `kazivukaL.qmz` | கழிவுகள், Chapter VI-A | `கழிவு_ஆக்கு` `தனி_வரம்பு` `கூட்டுக்_கழிவுகள்` `மீதமுள்ள_இடம்` `நன்கொடைக்_கழிவு` `கழிவுக்குரிய_வருமானம்` `அனுமதிக்கத்தக்க_கழிவு` `இழந்த_கழிவு` `வருமானத்தை_வட்டமிடு` `மொத்த_வரிக்குரிய_வருமானம்` |
| `mUlavari.qmz` | மூலவரி, tax deducted at source | `பிரிவு_ஆக்கு` `பிடிக்க_வேண்டுமா` `பிடித்த_அடிப்படை` `பயன்படும்_வீதம்` `வரி_நீக்கிய_அடிப்படை` `பிடித்தத்_தொகை` `ஆண்டுப்_பிடித்தம்` `முழு_மாதங்கள்` `பிடிக்காத_வட்டி` `செலுத்தாத_வட்டி` `சம்பள_பிடித்தம்` `வட்டித்_தொகை` |
| `muZvari.qmz` | முன்வரி, advance tax and the interest for | `மதிப்பிட்ட_வரி` `கடமை_உள்ளதா` `மூத்தவர்_விலக்கு` `முன்வரித்_தவணைகள்` `ஒற்றைத்_தவணை_அட்டவணை` `தவணை_வரை_தேவை` `தவணை_அட்டவணையை_ஆக்கு` `வட்டிக்குரிய_அடிப்படை` `மாதங்களாக` `தவணை_வட்டி` `குறைபாட்டு_வட்டி` `தாமதத்_தாக்கல்_வட்டி` `செலுத்த_வேண்டியது` |
| `nErativari_cOqaZY.qmz` | tests for the direct tax modules | — |
| `oqqivari.qmz` | ஒத்திவரி, deferred tax under Ind AS 12 | `வரி_அடிப்படை_சொத்து` `வரி_அடிப்படை_பொறுப்பு` `தற்காலிக_வேறுபாடு` `வேறுபாட்டு_வகை` `ஒத்திவரிக்_கணக்கு` `ஒத்திவரியைத்_திரட்டு` `அங்கீகரிக்கத்தக்க_சொத்து` `இழப்பின்_ஒத்திவரி` `ஈடுசெய்யலாமா` `ஒத்திவரி_மாற்றம்` `மொத்த_வரிச்_செலவு` `நிரந்தர_தாக்கம்` `வீத_ஒப்புரவு` |
| `varikkaNakku.qmz` | வரிக்கணக்கு, total income to tax payable | `அடிப்படை_வரி` `தள்ளுபடியைக்_கணக்கிடு` `மேல்வரி_படி` `மேல்வரி_விகிதம்` `மேல்வரி_தொகை` `விளிம்பு_நிவாரணம்` `கழிவு_வரி` `வரியை_வட்டமிடு` `வரியைக்_கணக்கிடு` `முறையை_ஆக்கு` `முறையின்_வரி` `முறைகளை_ஒப்பிடு` |
| `varumAZam.qmz` | வருமானம், head by head | `சம்பள_வருமானம்` `நிகர_ஆண்டு_மதிப்பீடு` `வாடகைச்_சொத்து_வருமானம்` `சொந்த_வீட்டு_வருமானம்` `வீட்டு_இழப்பை_ஈடுசெய்` `குறுகிய_காலமா` `குறியீட்டுச்_செலவு` `மூலதன_ஆதாயம்` `விலக்கிய_ஆதாயம்` `ஈட்டு_விதிகள்` `ஈடுசெய்ய_முடியுமா` `தலைப்பு_ஆக்கு` `ஈட்டைச்_செய்` `மொத்த_வருமானம்` `தலைப்பைத்_தேடு` `இழப்புத்_தலைப்புகள்` |

## uqavi — உதவி (worked examples)

`uqavi/` holds 76 runnable programs, one for each function above, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh nErativari
```
