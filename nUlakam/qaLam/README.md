# qaLam — தளம்

Redis, in words rather than command strings.

Each function names what it does — `சேமி`, `எடு`, `நீக்கு`, `ஒன்று_கூட்டு` — instead of asking the caller to remember SET, GET, DEL and INCR and to quote them correctly.

`எடு` answers `சரி(இன்மை)` for a key that is not there rather than a `தவறு`: a missing key is an ordinary answer to a cache lookup, not a failure.

## Files

| File | What it holds | Functions |
|---|---|---|
| `retis.qmz` | Redis, in words rather than command strings | `சேமி` `காலத்துடன்_சேமி` `எடு` `இருக்கிறதா` `நீக்கு` `ஒன்று_கூட்டு` `முன்_சேர்` `வரிசைப்_பகுதி` `உயிர்ப்பா` `இல்லையெனில்_இயல்பு` |
| `retis_cOqaZY.qmz` | tests for the client, against a server | `கொண்டுள்ளதா_எளிது` |

## uqavi — உதவி (worked examples)

`uqavi/` holds 10 runnable programs, one for each function above, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh qaLam
```
