# ezuqqu — எழுத்துப்பெயர்ப்பு

The romanization eTamil itself is written in, available to programs written in eTamil.

One Latin letter per Tamil letter, which is what makes the scheme invertible: `ezuqqu` is எழுத்து, and எழுத்து is `ezuqqu`. The three nasals stay distinct — **ண is `N`, ந is `n`, ன is `Z`** — because a scheme that blurs them cannot round-trip a Tamil word at all, and Tamil words are what this language is made of.

Until now the scheme lived only in `scripts/transliterate.py`, the tool CI audits the keyword table with. So the one thing a program written in eTamil could not do was ask the question eTamil is built on. `ezuqqu.qmz` is that scheme, in eTamil.

Every expected value in the tests is what `scripts/transliterate.py` produces, and the two implementations are checked against each other across all 203 Tamil keywords in the lexer. Two implementations of one scheme that disagreed would mean the language and its own audit disagreed.

## Files

| File | What it holds | Functions |
|---|---|---|
| `ezuqqu.qmz` | the scheme, both directions | `ஒலிபெயர்` `தமிழாக்கு` `திட்டப்படியா` |
| `ezuqqu_cOqaZY.qmz` | 33 tests, including the round trip | — |

## How it reads a word

A Tamil letter is one grapheme cluster, so `சரம்[இ]` yields கா whole rather than க and ா separately. The tables are therefore keyed by the cluster and built by joining the three source tables — vowels, vowel signs, consonants — rather than by walking code points the way the Python does. There are about three hundred clusters, and none of them is written out by hand: a second copy of the mapping would be a second thing to keep in step.

The one place the walk looks ahead is க்ஷ, which is a single letter in the scheme and two clusters on the screen.

Anything the scheme does not assign — a digit, an underscore, an ASCII letter — passes through unchanged in both directions. That is deliberate, and it is what makes the scheme usable as an audit: run a name through it and whatever comes back untouched is something the scheme never covered.

## uqavi — உதவி (worked examples)

`uqavi/` holds one runnable program per function, named after the function it
demonstrates. Each shows the ordinary use and the cases that are easy to get
wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching its
function fails rather than quietly going stale:

```bash
bash scripts/run_samples.sh ezuqqu
```
