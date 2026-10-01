# qaLam — தளம்

The two things a program keeps outside itself: a cache, and a queue.

Redis, in words rather than command strings.

Each function names what it does — `சேமி`, `எடு`, `நீக்கு`, `ஒன்று_கூட்டு` — instead of asking the caller to remember SET, GET, DEL and INCR and to quote them correctly.

`எடு` answers `சரி(இன்மை)` for a key that is not there rather than a `தவறு`: a missing key is an ordinary answer to a cache lookup, not a failure.

## A message broker, and what "sent" is allowed to mean

`ceyqi.qmz` sits on the AMQP builtins the way `retis.qmz` sits on `ரெடிஸ்_கட்டளை`: the protocol is the builtins, and what is here is the handful of shapes a program actually writes.

`ஒன்றைச்_செயலாக்கு` is the reason the file exists. It takes one message, hands it to the function you pass, and **acknowledges only if that function returned `சரி`**. Nothing is acknowledged before the work is done, so a program that stops half way leaves the message to be delivered again rather than having thrown it away. That discipline is easy to state and easy to forget at the fourth call site, so it is written once.

Its third argument is not a convenience. `மீண்டுமா` as `மெய்` puts a rejected message back — right for a failure that will pass later, and an endless loop for one that will not. As `பொய்` it dead-letters the message, or **discards it** if the queue names no dead-letter exchange. Neither is a safe default, so there is no default.

`docs/backend/AMQP.md` has the rest, including why every publish waits for the broker to confirm it *and* reports a message that routed nowhere as a failure.

## Files

| File | What it holds | Functions |
|---|---|---|
| `retis.qmz` | Redis, in words rather than command strings | `சேமி` `காலத்துடன்_சேமி` `எடு` `இருக்கிறதா` `நீக்கு` `ஒன்று_கூட்டு` `முன்_சேர்` `வரிசைப்_பகுதி` `உயிர்ப்பா` `இல்லையெனில்_இயல்பு` |
| `retis_cOqaZY.qmz` | tests for the client, against a server | `கொண்டுள்ளதா_எளிது` |
| `ceyqi.qmz` | a message broker (AMQP), the three things most callers do | `வரிசையைத்_தயாரி` `ஜேசானாக_அனுப்பு` `ஒன்றைச்_செயலாக்கு` |

## uqavi — உதவி (worked examples)

`uqavi/` holds 10 runnable programs for the Redis functions, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh qaLam
```

The broker functions have no `uqavi/` samples, because every one of them needs a broker and a sample that cannot run is a sample that goes stale unnoticed. `examples/api/ceyqi_varicY.qmz` is the worked example instead — skipped by `run_examples.sh` without `ETAMIL_TEST_AMQP`, and run for real by CI against a RabbitMQ service container.
