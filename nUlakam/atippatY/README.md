# atippatY — அடிப்படை

The fundamentals. Arrays, strings, records and arithmetic — the four things almost every other folder imports before it does anything of its own.

Nothing here knows about money, dates or accounting. That is deliberate: a helper that knows what a rupee is cannot be used to sort a list of pin numbers, and `kaNiqam.qmz` is imported by 57 files precisely because it does not.

Some of what used to live here now lives in the host. `பிரி`, `ஒன்றிணை` and `மாற்று` left `col.qmz`, and `புலம்_உள்ளதா` and `புலம்_அல்லது` left `poruL.qmz`, because a function defined here shadows a builtin of the same name. Each file records what went and why.

## Files

| File | What it holds | Functions |
|---|---|---|
| `aNi.qmz` | அணி (array) helpers | `உள்ளதா` `இடம்_காண்` `தலைகீழ்` `வெட்டு` `புலம்_எடு` `காலியா` `புலத்தால்_வடிகட்டு` `ஒவ்வொன்றுக்கும்` `வடிகட்டு` `மடி` `அணி_நிரப்பு` |
| `aNi_cOqaZY.qmz` | tests for map, filter and fold | `இரட்டி` `மேல்_உள்ளவை` |
| `col.qmz` | சரம் (string) helpers | `எழுத்து` `துண்டு` `தேடு` `கொண்டுள்ளதா` `தொடங்குகிறதா` `முடிகிறதா` `ஒழுங்கு` `திரும்பச்செய்` `இடமிருந்து_நிரப்பு` |
| `eNkaL.qmz` | எண்கள் (numbers): the operations on one or two of them | `முழுமதிப்பு` `சிறியது` `பெரியது` `முழு_எண்ணா` `மீதி` |
| `kaNiqam.qmz` | கணிதம் (math) helpers | — |
| `poruL.qmz` | பொருள் (record) helpers | `புலங்கள்` `மதிப்பீடுகள்` `காலியா_பதிவேடு` |
| `qokuppu.qmz` | தொகுப்பு (aggregates): reading one number out of many | `கூட்டு` `சராசரி` `மிகச்சிறியது` `மிகப்பெரியது` |
| `vikiqam.qmz` | விகிதம் (rates): percentages and arithmetic that lands on the paisa | `சதவீதம்` `வட்டக்_கழி` `குறையாக்_கழி` `வட்டப்_பங்கு` `வட்டப்_பெருக்கு` `நாள்_விகிதம்` `வட்ட_மாதங்கள்` |

`aNi.qmz`'s last three take a `செயல்` as a value — map, filter and fold. They were impossible until functions were values, and the loops written before them are left as they are.

## uqavi — உதவி (worked examples)

`uqavi/` holds 41 runnable programs, one for each function above, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh atippatY
```
