# cuwkam — சுங்கம்

Customs and trade documents.

The assessable value from price, freight and insurance, and the duty cascade where each tax is charged on a base that includes the one before it. Tariff headings and whether a code falls under one.

Also the e-way bill: whether a consignment needs one, how long it stays valid for the distance, and the shape of a GSTIN.

## Files

| File | What it holds | Functions |
|---|---|---|
| `cuwkam.qmz` | சுங்கம் (customs) and trade documents | `மதிப்பிடத்தக்க_மதிப்பு` `சுங்கக்_கணக்கு` `தலைப்பு_சரியா` `எண்களே` `அத்தியாயம்` `பொருந்துமா` `தேவையா` `செல்லுபடி_நாட்கள்` `வழிச்சீட்டு_சரிபார்` `ஜிஎஸ்டி_எண்_சரியா` `ஜிஎஸ்டி_மாநிலம்` `முதல்_இரண்டு` |
| `cuwkam_cOqaZY.qmz` | tests for the duty cascade, tariff headings and the e-way bill | — |

## uqavi — உதவி (worked examples)

`uqavi/` holds 12 runnable programs, one for each function above, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh cuwkam
```
