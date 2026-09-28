# nuNNaRivu — நுண்ணறிவு

Retrieval and language-model support, written in eTamil.

The same reasoning as the rest of `nUlakam`: if building an assistant for
eTamil required a different language, the claim that eTamil is sufficient for
the frameworks built on it would be false. Everything here is ordinary eTamil
over the host's arithmetic, strings and HTTP.

```etamil
இறக்கு "nUlakam/nuNNaRivu/coRpiri.qmz";
இறக்கு "nUlakam/nuNNaRivu/coRqEtal.qmz";

ஆவணங்கள் = [
    ஆவணம்_ஆக்கு("aNi", தேடல்_பதங்கள்("தலைகீழ் — reverse an array")),
    ஆவணம்_ஆக்கு("vari", தேடல்_பதங்கள்("வரி_தொகை — how much GST on an amount"))
];
குறியீட்டு = குறியீட்டு_ஆக்கு(ஆவணங்கள்);

ஒவ்வொரு பொருத்தம் இல் பொருத்தங்கள்(குறியீட்டு, தேடல்_பதங்கள்("reverse an array")) {
    அச்சு பொருத்தம்.குறி;                     // aNi
}
```

## What this is for

Answering "which part of the documentation answers this question?" — and then,
optionally, asking a model to phrase the answer. The two halves are separate on
purpose. Retrieval alone is useful and costs milliseconds; generation is the
slow part and must never be required for the library to be of use.

## Modules

| File | Contents |
|---|---|
| `coRpiri.qmz` | text into terms — `பிரிப்பான்கள்` `பிரிப்பானா` `இயல்பாக்கு` `பதங்கள்` `நிறுத்துப்_பதமா` `பொருளுள்ளவை` `தனிப்பதங்கள்` `தேடல்_பதங்கள்` |
| `eNNikkY.qmz` | counting terms — `ஆவணம்_ஆக்கு` `பத_அதிர்வெண்` `எத்தனை_முறை` `ஆவண_அதிர்வெண்` `தனித்தவை_உள்ளூர்` `சராசரி_ஆவண_நீளம்` `குறியீட்டு_ஆக்கு` |
| `coRqEtal.qmz` | relevance by shared words, BM25 — `கே1` `பி` `தலைகீழ்_அதிர்வெண்` `ஆவண_மதிப்பெண்` `பொருத்தங்கள்` `சிறந்தவை` |
| `muzuqqEtal.qmz` | full-text search on an index, BM25 through FTS5 — `பெயர்_சரியா` `சொல்_தப்பி` `முன்னொட்டாக்கு` `வினவல்_ஆக்கு` `வினவல்_வரிசை` `அட்டவணை_ஆக்கு` `ஆவணம்_இடு` `எடைப்_பட்டி` `முழுப்_பொருத்தம்` `ஆவண_எண்ணிக்கை` |
| `qicYyaZ.qmz` | vector arithmetic — `புள்ளிப்_பெருக்கம்` `திசை_அளவு` `ஒருமைப்படுத்து` `கோசைன்_ஒற்றுமை` `கூட்டுத்_திசையன்` `மடங்காக்கு` `சராசரித்_திசையன்` |
| `oRRumY.qmz` | finding by meaning — `பொதிந்த_ஆவணம்` `அண்மையவை` `அகலம்_ஒத்ததா` `வரம்புக்குள்_அண்மையவை` |
| `iNYppu.qmz` | merging rankings — `கே_மாறிலி` `பங்களிப்பு` `இணை_தரவரிசைகள்` `இரண்டை_இணை` `வகை_எடை_பயன்படுத்து` |
| `qaravaricY.qmz` | ranking scored results — `மதிப்பெண்_பெறு` `வரிசைப்படுத்து` `முதல்_சில` `வரம்புக்கு_மேல்` `மேலோங்கியதா` `இடம்_குறி` `மதிப்பெண்_சேர்` |
| `quNtAkkam.qmz` | splitting documents — `பத்திகளாக` `வரிகளாக` `அளவுக்கேற்பத்_துண்டாக்கு` `பத்திகளை_ஒன்றிணை` `ஆவணத்தைத்_துண்டாக்கு` |
| `utpoqippu.qmz` | text into vectors — `இயல்பு_முகவரி` `இயல்பு_மாதிரி` `உரை_தயாரி` `கோரிக்கை_உடல்` `பதிலைப்_பிரி` `பொதி` `ஒன்றைப்_பொதி` `சேவை_இயங்குகிறதா` |
| `urYyAkkam.qmz` | asking a model — `உள்ளூர்_வழங்குநர்` `இயல்பு_இணையவழி` `இயல்பு_மொழி_மாதிரி` `அதிகபட்சச்_சொற்கள்` `வழங்குநர்_ஆக்கு` `உள்ளூர்_வழங்குநர்_ஆக்கு` `திறவுகோல்_தேவையா` `தலைப்புகளை_ஆக்கு` `கோரிக்கை_உடலை_ஆக்கு` `பதிலிறுப்பைப்_பிரி` `உருவாக்கு` `கிடைக்கிறதா` |
| `kELvi.qmz` | putting the question — `அதிகபட்ச_மேற்கோள்கள்` `ஒரு_மேற்கோளின்_நீளம்` `மேற்கோள்_ஆக்கு` `சுருக்கு` `பின்புலம்_ஆக்கு` `அறிவுறுத்தல்` `தூண்டுதல்_ஆக்கு` `மேற்கோள்_இல்லாத_பதில்` |
| `aLavItu.qmz` | measuring whether it works — `சோதனை_ஆக்கு` `கண்டுபிடித்த_இடம்` `தலைகீழ்_இடம்` `சராசரி_தலைகீழ்_இடம்` `எத்தனை_முதல்_சிலவற்றில்` `கண்டுபிடிப்பு_விகிதம்` `முடிவுகளைத்_தொகு` |
| `matakkY.qmz` | logarithms — `இரண்டின்_மடக்கை` `இரண்டின்_அடிப்படை_மடக்கை`. `இயற்கை_மடக்கை` and `பத்தின்_மடக்கை` are host builtins now |

## How they fit together

```
                    coRpiri  ──terms──►  eNNikkY  ──counts──►  coRqEtal ─┐
  a document                                                 (by word)   │
       │                                                                 ├─► iNYppu ─► qaravaricY
       │                                                                 │   (merge)     (rank)
  quNtAkkam ──chunks──► utpoqippu ──vectors──►  oRRumY  ──────────────────┘
                                             (by meaning)
                                                                         │
                                    kELvi ◄── the winning chunks ────────┘
                                      │
                                      ▼
                                  urYyAkkam ──► the phrased reply
                                                        │
                                              aLavItu ◄─┘  measures the whole path
```

`matakkY.qmz` sits underneath `coRqEtal.qmz`; `qicYyaZ.qmz` underneath
`oRRumY.qmz`.

## Why both kinds of search

They fail in different places, and each covers the other.

Word matching cannot find what shares no word with the question. "How do I
connect to postgres" and a function named `qaLam_iNY` have nothing in common
as text, so `coRqEtal.qmz` will never return it however it is tuned.

Meaning matching does not know what is rare. It has no notion that `அணி`
distinguishes a document while `the` does not, so a question whose exact term
appears in one place can still be answered with something vaguely related.

`iNYppu.qmz` runs both and rewards agreement — which is also why adding a
third, weaker retriever tends to make results *worse*: it dilutes the
agreement of the two that work.

## The constants, and how to change them

Each is a function rather than a literal so it is named where it is used, and
none should be changed without measuring:

| Constant | Where | What it controls |
|---|---|---|
| `கே1` = 1.2 | `coRqEtal.qmz` | how fast repeated terms stop helping |
| `பி` = 0.75 | `coRqEtal.qmz` | how strongly document length is normalised |
| `கே_மாறிலி` = 60 | `iNYppu.qmz` | how much better rank 1 is than rank 2 |
| `அதிகபட்ச_மேற்கோள்கள்` = 3 | `kELvi.qmz` | how much material the model is shown |
| `அதிகபட்சச்_சொற்கள்` = 160 | `urYyAkkam.qmz` | reply length, and most of the wait |

`aLavItu.qmz` exists so that changing any of them is an experiment rather than
an opinion. Keep two sets of questions: one written while diagnosing failures,
and one written independently and phrased differently. When they disagree,
believe the second — the first has usually had its wording copied into the
documentation it is scoring.

## Talking to a model

`utpoqippu.qmz` and `urYyAkkam.qmz` default to Ollama on `127.0.0.1`. Nothing
leaves the machine, and no account is needed to use any of this.

`urYyAkkam.qmz` also accepts other providers through `வழங்குநர்_ஆக்கு`, which
takes a key supplied by whoever is running the program. There is no default key
anywhere in this folder and no place one could be compiled in. A provider
needing a key that has not been given one fails before the request is made, so
the reason is "no key" rather than whatever the service returns.

Two mistakes are silent and worth stating plainly:

- **The same embedding model must index the documents and embed the questions.**
  Vectors from two models are unrelated coordinate systems. `அகலம்_ஒத்ததா` is
  there to be asked, because otherwise every comparison is skipped, the result
  is an empty list, and an empty list looks exactly like "nothing matched".
- **The text must be prepared identically both times.** Indexing with a title
  prepended and querying without it places the two sides by different rules.

## Reserved words

Several plain words cannot be used as names. `சொல்`, `உரை`, `தரவு`, `தலைப்பு`,
`இடம்`, `வரிசை`, `வரம்பு`, `முகவரி`, `பதில்`, `மதிப்பீடு`, `அணி` and `பொருள்`
are all keywords. Hence `பதம்` for a term, `சரம்` for a string, `தலைப்புரை`
for a title, `இடநிலை` for a position, `எல்லை` for a limit, `இணையவழி` for an
address and `பதிலிறுப்பு` for a response.

A reserved word used as a name does not fail where it is declared — it fails at
the first use, naming the word rather than the declaration.

## Precision

`இயற்கை_மடக்கை`, `பத்தின்_மடக்கை` and `வர்க்கமூலம்` are host builtins, computed on the
Decimal itself rather than through f64. They were series written in eTamil
until the host had them, and the series were accurate to about 28 digits, so
going through a binary float would have been a regression dressed as a
speed-up.

Two artifacts this page used to warn about are gone with them. `√25` is `5`,
not `5.000000000000000000000288615`, and `log10(1000)` is `3`, not
`3.0000000000000000000000000006` — the last needed its own builtin, because
`ln(x)/ln(10)` is not exact.

What is left in `matakkY.qmz` is the base-2 wrapper and the `ln 2` constant it
divides by, neither of which was ever the expensive part.

`coRqEtal.qmz` and `muzuqqEtal.qmz` compute the same BM25 and differ in where
the work happens. `coRqEtal.qmz` scores documents it is handed, which is right
when they are already in hand and few. `muzuqqEtal.qmz` keeps them in an FTS5
table and lets SQLite hold the inverted index, so a query touches the postings
for its own words rather than every document — which is what keeps a search
fast as a collection grows.

## uqavi — உதவி (worked examples)

`uqavi/` holds 96 runnable programs, one for each function above, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh nuNNaRivu
```
