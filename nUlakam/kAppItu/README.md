# kAppItu — காப்பீடு

Insurance: policy, premium and claim.

Premiums at a percentage or per thousand, loaded and discounted, and pro-rated for a short period. On the claim side, the excess the insured bears and the average clause, which reduces a claim in proportion when the sum insured was less than the true value.

`கோரல்_தீர்வு` applies both in the right order, which is the part that is easy to get wrong.

## Files

| File | What it holds | Functions |
|---|---|---|
| `kAppItu.qmz` | காப்பீடு (insurance): policy, premium, claim | `முனைமம்` `ஆயிரத்திற்கு_முனைமம்` `ஏற்ற_இறக்கத்துடன்` `குறுகிய_காலம்` `கழிவுக்குப்_பின்` `சராசரி_விதி` `கோரல்_தீர்வு` `கோரல்_இல்லா_சலுகை` `நிலுவைக்_கோரல்கள்` |
| `kAppItu_cOqaZY.qmz` | tests for policy, premium and claim | — |

## uqavi — உதவி (worked examples)

`uqavi/` holds 9 runnable programs, one for each function above, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh kAppItu
```
