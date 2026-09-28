# vawki — வங்கி

Banking: interest, loans, and asset classification.

Simple, daily and compound interest, with the day-count convention as an argument rather than an assumption. Loan instalments and the amortisation schedule, and what it costs to settle early.

`coqqu.qmz` classifies an account by how long it has been overdue and says what has to be provided against it — with the rules dated, because the rules change and a past classification has to stay reproducible.

## Files

| File | What it holds | Functions |
|---|---|---|
| `coqqu.qmz` | சொத்து வகைப்பாடு (asset classification) | `விதிமுறைகளை_ஏற்று` `வகைப்படுத்து` `ஒதுக்கீடு` `சரிபார்க்கப்படாதவை` `தொடங்குகிறதா_எளிது` |
| `coqqu_cOqaZY.qmz` | tests for asset classification | `வகை_பெயர்` |
| `kataZ.qmz` | கடன் (loans): the instalment, and the schedule | `மாத_விகிதம்` `மாதத்_தவணை` `தவணை_அட்டவணை` `மொத்த_வட்டி` `மொத்தத்_திருப்பி` `முன்கூட்டியே_அடைத்தால்` |
| `vatti.qmz` | வட்டி (interest) | `நாட்கள்` `ஆண்டுப்_பங்கு` `அடுக்கு` `எளிய_வட்டி` `நாளாந்த_வட்டி` `கூட்டு_வட்டி` `முதிர்வுத்_தொகை` |
| `vawki_cOqaZY.qmz` | tests for interest and loans | — |

## uqavi — உதவி (worked examples)

`uqavi/` holds 18 runnable programs, one for each function above, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh vawki
```
