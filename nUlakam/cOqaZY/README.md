# cOqaZY — சோதனை

The test framework, written in eTamil like everything it tests.

A run is a record that accumulates passes and failures; each assertion takes the run and gives back a new one, so a suite is a fold rather than a sequence of side effects. `சோதனை_முடிவு` prints the summary and exits non-zero if anything failed — reporting a failure and exiting 0 tells the truth to a reader and a lie to everything that runs the suite.

Every `*_cOqaZY.qmz` file elsewhere in nUlakam imports this one.

## Files

| File | What it holds | Functions |
|---|---|---|
| `cOqaZY.qmz` | சோதனை (tests), written in eTamil | `சோதனை_தொடக்கம்` `உறுதிசெய்` `சமம்` `வேறுபடு` `சேர்_ஓட்டம்` `சோதனை_முடிவு` |

## uqavi — உதவி (worked examples)

`uqavi/` holds 6 runnable programs, one for each function above, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh cOqaZY
```
