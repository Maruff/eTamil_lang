# upi — ஒருங்கிணைந்த பணப்பரிமாற்றம்

Unified payments: addresses, pay links and payment state.

`vilAcam.qmz` validates a virtual payment address, formats an amount the way the specification wants it, and builds and reads a pay link.

`nilYmY.qmz` is the state machine. It exists because the interesting question is not what state a payment is in but which moves between states are legitimate — a payment that has settled must not go back to pending, and `நகர்த்து` refuses rather than allowing it.

## Files

| File | What it holds | Functions |
|---|---|---|
| `nilYmY.qmz` | where a UPI payment has got to | `நிலைமைகள்` `அறியப்பட்டதா` `பணம்_வந்ததா` `சரிபார்க்கவா` `முடிந்ததா` `நகர்வு_சரியா` `நகர்த்து` |
| `upi_cOqaZY.qmz` | tests for addresses, pay links and payment states | — |
| `vilAcam.qmz` | UPI virtual payment addresses, and the pay link | `முகவரி_சரியா` `கை_பகுதி` `தொகை_சரியா` `தொகை_உரை` `உரை_மறை` `பணம்_இணைப்பு` `விருப்பத்தைச்_சேர்` `இணைப்பைப்_படி` |

## uqavi — உதவி (worked examples)

`uqavi/` holds 15 runnable programs, one for each function above, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh upi
```
