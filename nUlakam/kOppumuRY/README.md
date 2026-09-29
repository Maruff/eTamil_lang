# kOppumuRY — கோப்புமுறை

The file system: paths as text, and directories as listings.

Paths are written with `/` on every platform, the way an `இறக்கு` is, so the
same program reads the same on Windows and on Linux. `பாதை_இயல்பாக்கு` is what
makes that true of a path that arrived from somewhere else — a shell, a
configuration file, another tool — and everything here normalises before it
looks at a path rather than trusting how it was spelled.

The path functions touch no disk. `பாதை_இணை`, `அடிப்பெயர்`, `கோப்பக_பெயர்`,
`நீட்சி` and `பெயர்_மட்டும்` are string work, and they answer for a path that
does not exist. Only `கோப்பகமா`, the listing and the walk ask the host anything.

A walk skips what it cannot read rather than failing, which is what lets it run
over a tree with a permission-denied corner in it. A listing of a directory that
is not there is a `தவறு`, because "nothing in it" and "no such directory" are
different answers and a caller collecting a corpus needs to tell them apart.

## Files

| File | What it holds | Functions |
|---|---|---|
| `kOppumuRY.qmz` | கோப்புமுறை (the file system): paths, directories, walking | `பாதை_இணை` `அடிப்பெயர்` `கோப்பக_பெயர்` `நீட்சி` `பெயர்_மட்டும்` `பாதை_இயல்பாக்கு` `கோப்பகமா` `வினாடியை_நாளாக` `கோப்பக_பட்டியல்` `கோப்பக_நட` `நீட்சியால்_கோப்புகள்` `துண்டு_அணி` |
| `kOppumuRY_cOqaZY.qmz` | tests: joining and splitting, existence and kind, listing and walking | — |

## uqavi — உதவி (worked examples)

`uqavi/` holds 12 runnable programs, one for each function above, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh kOppumuRY
```
