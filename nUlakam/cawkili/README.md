# cawkili — சங்கிலி

Hyperledger Fabric, through a REST gateway.

Queries and submissions to a chaincode, with the gateway's paths configurable because different gateways lay them out differently.

The part worth reading is `மோதலா`, which distinguishes a read-write conflict — two transactions touching the same key, where retrying is correct — from a real failure, where retrying just repeats the error. `மீண்டும்_சமர்ப்பி` uses it.

## Files

| File | What it holds | Functions |
|---|---|---|
| `fabric.qmz` | Hyperledger Fabric, through a REST gateway | `நுழைவு` `பாதையை_அமை` `முழு_முகவரி` `உடலைக்_கட்டு` `விடையைப்_படி` `மதிப்பிடு` `சமர்ப்பி` `மோதலா` `மீண்டும்_சமர்ப்பி` |
| `fabric_cOqaZY.qmz` | tests for the Fabric gateway client | — |

## uqavi — உதவி (worked examples)

`uqavi/` holds 9 runnable programs, one for each function above, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh cawkili
```
