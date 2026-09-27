# What building nuNNaRivu found missing in eTamil

Written while implementing `nUlakam/nuNNaRivu` — thirteen modules of retrieval
and language-model support, 88 functions. Everything here was hit while
writing ordinary library code, and every number was measured on this machine
with `etamil_compiler/target/release/etamil.exe`.

The library works. This is the list of what it cost to make it work, and what
would make the next library cheaper.

## The finding that decides an architecture

**Records cannot be used as maps.** Not "are slow as maps" — cannot be used.
Two facts combine badly:

Reading a key that is absent is a fatal runtime error, not `இன்மை`:

```
பதிவேடு = { a: 1 };
அச்சு பதிவேடு["nope"];
✗ Runtime error: புலம் 'nope' இல்லை  (no field 'nope' on this record)
```

So every read of a possibly-absent key must be guarded by
`புலம்_உள்ளதா`. And `புலம்_உள்ளதா` has no primitive to call: `nUlakam/poruL.qmz`
implements it by iterating the record's field names, because nothing else is
available. Building a record of *n* keys and then asking *n* times whether a
key is present:

| keys | time | net of the ~1.0 s startup |
|---|---|---|
| 100 | 1.12 s | 0.12 s |
| 200 | 2.16 s | 1.16 s |
| 400 | 8.06 s | 7.06 s |
| 800 | 62.63 s | 61.63 s |

Doubling the size multiplies the time by six to nine. That is worse than
quadratic, which suggests iterating a record materialises its key list per
call rather than merely walking it.

### What that cost

`coRqEtal.qmz` scores documents by BM25, which needs a term → document-count
map over the whole corpus. Built in eTamil that map is the table above. At
paRY's real corpus of 2383 chunks, indexing did not finish in ten minutes and
had to be killed. Phase timings at n=400 documents:

| phase | n=400 |
|---|---|
| tokenize | 2.8 s |
| append to an array | 2.5 s |
| **build the index** | **53.4 s** |

And end to end:

```
n=50   3.1 s      n=200  13.0 s
n=100  3.6 s      n=400  91.4 s
```

This is the single reason paRY's retrieval core cannot currently be ported to
eTamil wholesale. Fusion and ranking over a short candidate list run in about
147 ms and are perfectly usable; it is only the corpus-wide frequency map that
is out of reach.

### A wrong diagnosis, recorded because it was costly

The first explanation was that records copy on write. It is false, and the
microbenchmark says so plainly — writes are flat:

| | 250 | 500 | 1000 | 2000 |
|---|---|---|---|---|
| record, one write per key | 1.00 s | 1.15 s | 1.24 s | 1.17 s |
| array, one `இணை` per item | 1.18 s | 0.99 s | 1.14 s | 1.02 s |

Both are startup-dominated and flat to 2000 elements. **Writing is fine.
Asking whether a key exists is what costs.** Anyone optimising this should
start at `புலம்_உள்ளதா`, not at assignment.

### What would fix it

A host builtin for key existence, and ideally one for "the field, or this
default" — the two operations `poruL.qmz` already defines in eTamil because it
has nothing to call. Both are one hash lookup in the interpreter. That single
change turns BM25 indexing from super-quadratic into linear and makes the
whole of `nuNNaRivu` usable at corpus scale.

Making an absent-key read return `இன்மை` would also work, but it trades a loud
failure for a quiet one, and the loud one has caught real mistakes.

## String building is quadratic

`coRpiri.qmz` originally normalised text by walking it character by character
and accumulating with `&`:

```etamil
(எண்ணி < மொத்தம்) சுற்று {
    விடை = விடை & ஒரு_எழுத்து;      // copies the whole string each time
    எண்ணி = எண்ணி + 1;
}
```

Rewritten to call the host `மாற்று` once per separator — 42 host calls per
document instead of one interpreted pass with a copy per character — it is fast
and produces identical output.

Correcting an earlier version of this page: this was blamed for the corpus
indexing that never finished, and it was not the cause. That was
`புலம்_உள்ளதா`, above, throughout. Measured on its own, `&` in a loop is
quadratic but only bites above about fifty thousand characters — 0.27s to build
50k, 0.88s for 100k, 4.88s for 200k — which is well past anything the tokenizer
was doing per document.

`nUlakam/col.qmz` records the same lesson for `பிரி` and `ஒன்றிணை`, which were
moved into the host after costing "14 seconds over 8 KB". The pattern is
established: **any loop that accumulates a string is a bug waiting for a large
input.** A string builder, or a host `ஒன்றிணை` over an array built by `இணை`,
is the workaround; the latter is what should be reached for today.

## The eTamil JSON parser does not scale

`ஜேசான்_படி` in `nUlakam/jEcAZ.qmz` is a hand-written recursive parser, and it
is correct. On a 624 KB corpus it did not finish in ten minutes.

The same data as tab-separated text, split with the host `பிரி`:

```
read 624 KB and split 2383 rows into 4 fields each   1.4 s
```

This is not a criticism of `jEcAZ.qmz` — it is the string and record costs
above, surfacing in the place that exercises them hardest. But it means JSON
is currently unusable as an interchange format for anything but small
payloads, which matters because JSON is what every HTTP API speaks. A host
JSON parser would remove the problem and would also let `utpoqippu.qmz` read
an embedding reply of any size.

## Missing from the language

Each of these had to be written or worked around while building the library.

**No logarithm.** `matakkY.qmz` exists only because BM25's rarity weight needs
`ln`, and the host has no logarithm, exponential or power. It is implemented by
argument reduction and an artanh series, accurate to about 28 digits —
`ln(10)` returns `2.3025850929940456840179914545`. `வர்க்கமூலம்` in
`kaNiqam.qmz` exists for the same reason and by the same method.

**No sort.** `வரிசைப்படுத்து` in `qaravaricY.qmz` is an insertion sort. That is
the right choice for ranking tens of results and the wrong one for anything
larger, and every library that needs ordering will write it again.

**No command-line arguments.** A program cannot see its own argv. `சூழல்` works
as a substitute, so `nuNNaRivu`'s callers pass input through the environment,
but that is a workaround and it does not compose — an argument containing a
newline or a very long one is awkward where argv would be ordinary.

**No structured output from the runner.** `etamil run` prints a banner, the
program's output, and a completion line, all on stdout. A program meant to be
called by another program has to be parsed out of that. `--check` is
errors-only; there is no equivalent for running. A `--quiet` that emits only
what the program printed would make eTamil usable as a subprocess.

**Startup is ~1.0 s** with `nuNNaRivu` imported. That is small for a script and
decisive for a request path: it rules out invoking eTamil per query and forces
a long-running `--server` process instead. Worth knowing before designing
around it.

## Inconsistencies that cost debugging time

**`வர்க்கமூலம்` returns a Result; arithmetic does not.** So `திசை_அளவு` in
`qicYyaZ.qmz` reads

```etamil
திரும்பு இயல்பு(வர்க்கமூலம்(கூட்டல்), 0);
```

where the sum of squares can never be negative and the failure is unreachable.
The alternative was to propagate a Result through every vector operation for a
case that cannot arise. The first version of the file divided by the Result
directly and failed with "division by zero", which names neither the cause nor
the file.

**Decimal results are not exact for derived functions.** `log10(1000)` is
`3.0000000000000000000000000006` and `√25` is `5.000000000000000000000288615`.
Correct to far more digits than anyone needs, but `== 3` is false. Anything
built on a series must be compared with a tolerance, and that is not obvious
from the call site.

## Naming: 203 reserved words, and they are common nouns

The lexer reserves 203 Tamil words, and they include much of the vocabulary a
library wants: `சொல்`, `உரை`, `தரவு`, `தலைப்பு`, `இடம்`, `வரிசை`, `வரம்பு`,
`முகவரி`, `பதில்`, `மதிப்பீடு`, `அணி`, `பொருள்`, `எண்`, `நிலை`, `தொகை`.

Writing `nuNNaRivu` hit five of them. The file now says `பதம்` for a term,
`சரம்` for a string, `தலைப்புரை` for a title, `இடநிலை` for a position, `எல்லை`
for a limit, `இணையவழி` for an address and `பதிலிறுப்பு` for a response — none
of which is the word a Tamil speaker would reach for first.

**The message did not explain itself.** Correcting an earlier version of this
page: the position was never wrong — the error lands on the declaration. What
it did not say is *why*, and for an assignment it did not mention names at all
(`வரிசை = 5;` reported "a statement was expected"). The parameter case read:

```
✗ வரி 41, நெடுவரிசை 18: a parameter name எதிர்பார்க்கப்பட்டது, 'இடம்' கிடைத்தது
```

Each one cost a compile-run cycle to identify. `col.qmz` and `poruL.qmz` both
carry comments explaining which word they could not use, which suggests this is
a standing tax rather than a one-off.

What would help, in order of how much: fewer hard reservations (many of these
are SQL and domain keywords that could be contextual); a message that names the
cause; or a lint that lists collisions in a file before compiling it. A
twenty-line script doing the last of these paid for itself immediately while
writing this library.

## No module system

Two problems, both hit.

**Names collide across modules.** `nuNNaRivu` defined `தேடு`; `col.qmz` already
exports a `தேடு` that finds a substring and returns a position. A program
importing both gets whichever loaded last, and the failure surfaces three
calls away:

```
✗ Runtime error: இதை சுற்ற முடியாது  (cannot iterate over a number)
```

Nothing points at the collision. The fix was to rename to `பொருத்தங்கள்`, and
the way it was found was a script comparing every new function name against the
708 already in `nUlakam`.

**Nothing can be private.** `eNNikkY.qmz` needs a "distinct values" helper and
deliberately does not import `coRpiri.qmz`'s version, because counting should
not depend on a particular tokenizer. So `தனித்தவை_உள்ளூர்` is exported into the
global namespace with a name that says "local" and is not. Every helper a
module needs becomes part of the library's surface, which is also why the
reserved-word and collision problems keep growing.

A namespace on import — `இறக்கு "…" ஆக த` and then `த.தேடு` — would solve both.

## Tooling observations

**`generate_editor_support.py` globs the working tree, not the commit.**
Regenerating while another branch has uncommitted work in `nUlakam` or
`interpreter.rs` silently bakes that work into the committed artifacts, and CI's
drift gate then fails for a reason that has nothing to do with the change.
Generating from a copy built out of `HEAD` plus only the new files is the
workaround. Reading from `git show HEAD:…` by default, or refusing to run with
a dirty tree unless forced, would remove the trap.

**`check_names.py` needs an allowlist entry per external identifier.**
`utpoqippu.qmz` reads Ollama's `embeddings` field, which is not a Tamil name
gone wrong and cannot be renamed. That is exactly what `ALLOW` is for, but it
means every library talking to an outside API edits a shared script.

## What has since been fixed

All of the following are done, measured, and covered by tests. The compiler
suite is 529 tests and every example still behaves as expected.

| | Fix | Before | After |
|---|---|---|---|
| 1 | `புலம்_உள்ளதா` / `புலம்_அல்லது` builtins | 62.63 s at n=800 | **1.89 s** |
| | BM25 index over 2383 chunks | never finished | **99.2 s** |
| 2 | `ஜேசான்_படி` / `ஜேசான்_ஆக்கு` builtins | 624 KB never finished | **1.31 s** |
| 3 | `ConcatVar`: `x = x & e` appends in place | 4.88 s for 200k chars | flat to 400k |
| 4 | `வர்க்கமூலம்` `இயற்கை_மடக்கை` `இயற்கை_அடுக்கு` `அடுக்கேற்று` `பத்தின்_மடக்கை` | series in eTamil | host, exact |
| 5 | Two imported modules may not define one name | silent, wrong function won | refused, both files named |
| 6 | The error says the word is reserved | "a statement was expected" | names the cause |
| 7 | `வரிசையாக்கு` / `புலத்தால்_வரிசையாக்கு` builtins | insertion sort per library | one host call |

Item 4 turned out to be a correctness fix as much as a speed one. `f64` would
have been a regression — the hand-written series were accurate to about 28
digits — so these are computed on the Decimal itself. Both precision artifacts
this page complained about are gone: `√25` is `5` rather than
`5.000000000000000000000288615`, and `log10(1000)` is `3` rather than
`3.0000000000000000000000000006`. The last needed its own builtin, because
`ln(x)/ln(10)` is not exact.

Item 3 is the same optimisation `x = இணை(x, v)` already had, applied to `&`.

Item 5 fixes the half that was dangerous — a collision is now refused with both
file names rather than silently resolved. Namespaced imports and private
helpers are **not** done: that is a syntax change affecting every import site,
and the flattening would have to be reworked. The remaining cost is that every
helper a module needs is still part of the library's public surface.

Superseded eTamil versions were deleted rather than left to delegate, because a
function defined in `nUlakam` shadows a builtin of the same name — the rule
`col.qmz` already records for `பிரி` and `ஒன்றிணை`. `jEcAZ.qmz` went from 340
lines to 67.

One thing preserved rather than decided: `ஜேசான்_ஆக்கு` writes `null` for
`சரி`, `தவறு` and a `செயல்`, and the eTamil version's own comment argued for
refusing instead while the code did not. The builtin keeps the existing
behaviour and records the contradiction. Changing it is a decision about the
library's contract.

## Summary, most valuable first

| | Fix | Effect |
|---|---|---|
| 1 | Host builtin for record key existence and lookup-with-default | Makes maps usable; turns BM25 indexing from super-quadratic to linear |
| 2 | Host JSON parse and serialise | Makes JSON usable as interchange; fixes any HTTP client with a large payload |
| 3 | A string builder, or guidance to use `ஒன்றிணை` over an array | Removes the commonest quadratic in library code |
| 4 | `ln`, `exp`, `pow` in the host | Removes two hand-written series from `nUlakam` |
| 5 | Module namespaces on import | Removes name collisions and lets helpers be private |
| 6 | Fewer hard-reserved words, or an error at the declaration | Removes a recurring tax on every new library |
| 7 | A sort builtin | Stops every library writing one |
| 8 | `--quiet` on the runner, and argv | Makes eTamil usable as a subprocess |
| 9 | Generator reads `HEAD`, not the working tree | Stops one branch's artifacts capturing another's work |

Items 1 and 2 are the two that currently decide what can and cannot be written
in eTamil. The rest are friction.
