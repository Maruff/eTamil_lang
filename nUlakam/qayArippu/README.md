# qayArippu — தயாரிப்பு

Product management: the versions a product ships, how long each is supported, and the features promised to customers for a date.

A version is major.minor.patch with the major as the platform, and versions compare as numbers — 6.10.0 comes after 6.9.4, which a text comparison gets backwards. A version is current until one supersedes it and supported for a window after. A release commitment is judged on two questions, and `vAkkuRuqi.qmz` answers both from the register: was it delivered by the date, and when it slipped, was the customer told before the target passed rather than after.

Lifecycles here are held in `நடப்பு_நிலை`, the field `../qittam/nilYmARRam.qmz` moves, so a team that wants a register's transitions enforced writes the machine as data and uses the same calls as change control.

## Files

| File | What it holds | Functions |
|---|---|---|
| `paqippu.qmz` | பதிப்பு: versions and their support life | `இயல்பு_ஆதரவு_மாதங்கள்` `பதிப்பைப்_படி` `பதிப்பை_எழுது` `பதிப்புகளை_ஒப்பிடு` `பதிப்பை_உயர்த்து` `அதே_தளமா` `ஆதரவு_நிலை` |
| `qayArippu_cOqaZY.qmz` | tests for the product management modules | — |
| `vAkkuRuqi.qmz` | வாக்குறுதி: release commitments to customers | `வாக்குறுதி_ஆக்கு` `உரிய_நாள்` `வாக்குறுதி_திறந்ததா` `வாக்குறுதி_கடந்ததா` `அறிவிப்பு_தேவையா` `அறிவிப்பு_தாமதமா` `தெரிவித்ததாகப்_பதி` `வாக்குறுதி_எண்ணிக்கைகள்` |

## uqavi — உதவி (worked examples)

`uqavi/` holds one runnable program for each function above, named
after the function it demonstrates. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh qayArippu
```
