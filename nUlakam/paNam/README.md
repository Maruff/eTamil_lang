# paNam — பணம்

Money: how it is stored, and how it is shown.

`kAcu.qmz` keeps amounts as whole paise, which is the only representation that survives being added up. Splitting is the part worth reading: `சமமாகப்_பிரி` hands the odd paisa to the earliest shares rather than rounding each share independently, because ten rupees three ways is 3.34, 3.33, 3.33 — not 3.33 three times with four paise quietly missing from the ledger.

`paNam.qmz` is the other half: grouping digits the Indian way, the rupee sign, lakhs and crores.

## Files

| File | What it holds | Functions |
|---|---|---|
| `kAcu.qmz` | காசு: money as whole paise | `ரூபாயும்_பைசாவும்` `ரூபாயாக` `காசு_உரை` `காசு_கூட்டு` `காசு_கழி` `காசு_மடங்கு` `காசு_கூட்டல்` `விழுக்காடு_காசு` `சமமாகப்_பிரி` `விகிதத்தில்_பிரி` |
| `kAcu_cOqaZY.qmz` | tests for money as whole paise | — |
| `paNam.qmz` | பணம் (money) formatting | `குழுக்கள்` `காசு_வடிவம்` `ரூபாய்` `காசாக` `லட்சம்` `கோடி` |

## uqavi — உதவி (worked examples)

`uqavi/` holds 16 runnable programs, one for each function above, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh paNam
```
