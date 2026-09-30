# qanqi — தந்தி

SWIFT MT: the format interbank payments still travel in.

தந்தி is the old word for a telegram, which is what MT descends from and still looks like — blocks in braces, and inside block 4 a list of `:tag:value` lines.

## What it handles, and what it does not

The **block and field structure**, which every MT shares: splitting the blocks, reading the fields in order, pulling the sender, receiver and message type, and taking field 32A apart.

It does **not** validate an MT103 against its network rules. That is a rulebook per message type and several hundred pages of it, and a library that validated half of it would be trusted for the half it does not do.

**MX — ISO 20022 XML — is a different format under the same name** and is not here.

## Two things that are easy to get wrong

**The fields are an array, not a record.** A statement repeats `:61:` once per entry, and a record would keep the last one and silently lose the rest of the month. `புலத்தைத்_தேடு` gets the first; `எல்லாப்_புலங்களும்` gets all of them.

**A SWIFT amount writes its decimal mark as a comma**, always present, and never groups thousands. `1234,56` read by something expecting a point becomes either an error or — worse — `123456`. `தொகையைப்_படி` refuses a point outright, because here it is neither a decimal mark nor a thousands mark, and `தொகையை_எழுது(1234)` gives `1234,` rather than `1234`.

Blocks 3 and 5 hold braces of their own, so the end of a block is found by counting depth. Stopping at the first `}` cuts block 3 in half.

`பெறுநர்` answers only for an input message, whose block 2 begins `I`. An output block 2 begins `O` and carries the sender's input reference in a different layout, so reading twelve characters from the same place would return a plausible BIC that is not the receiver. It says it cannot answer instead.

## Files

| File | What it holds | Functions |
|---|---|---|
| `qanqi.qmz` | SWIFT MT blocks, fields and amounts | `தொகுதிகளைப்_படி` `அனுப்புநர்` `பெறுநர்` `செய்தி_வகை` `புலங்களைப்_படி` `புலத்தைத்_தேடு` `எல்லாப்_புலங்களும்` `தொகையைப்_படி` `தொகையை_எழுது` `நாள்_நாணயம்_தொகை` `செய்தியை_ஆக்கு` |
| `qanqi_cOqaZY.qmz` | 37 tests, over a real MT103 and a statement | — |

## uqavi — உதவி (worked examples)

```bash
bash scripts/run_samples.sh qanqi
```

`qokuqikaLYp_pati.qmz` takes an MT103 apart; `qokYyYp_pati.qmz` is the comma.
Unlike the older packages this is not yet one per function.
