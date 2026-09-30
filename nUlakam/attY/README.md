# attY — அட்டை

ISO 8583: the message every card transaction in the country is carried in.

Four digits of message type, a bitmap saying which fields are present, and then the fields — each with its own length rule. `attY.qmz` reads one and writes one.

## What it handles, and what it does not

The **ASCII variant** — a hex bitmap and decimal digits as text — which is what an acquirer or a switch normally presents. The binary and BCD packings of the same standard are the same field table over a different encoding, and they are not here: claiming a packing that has never been tested against a real switch would be worse than saying so.

Field definitions cover the data elements an acquirer interface actually sends. **A field with no definition stops the parse** rather than being guessed at — an unknown length mis-reads everything after it, which in a settlement file means every transaction after the first odd one.

Nothing here opens a socket. A message is text in and text out, so a test is a message and so is a production log line.

## The bitmap

Sixty-four bits as sixteen hex characters, one per field. Bit 1 is not a field: it says a secondary bitmap follows, carrying fields 65 to 128. `பிட்_வரைபடத்தை_ஆக்கு` sets that bit itself when it needs to — a message whose bit 1 disagrees with its own length is one a switch rejects without explaining.

eTamil has no bit operators, so a hex digit becomes a number and its four bits come out by division and remainder. That is the same arithmetic a shift would do, written the way this language can write it.

## Only four codes are approvals

`ஏற்கப்பட்டதா` answers true for `00`, `08`, `10` and `11` and for nothing else — an unrecognised code is a decline. Treating an unknown response as success is how a shop ships goods that were never paid for, which is the same mistake `upi/nilYmY.qmz` exists to prevent on the other rail.

## Files

| File | What it holds | Functions |
|---|---|---|
| `attY.qmz` | ISO 8583 messages | `செய்தி_வகையைப்_படி` `பதிலா` `பிட்_வரைபடத்தைப்_படி` `பிட்_வரைபடத்தை_ஆக்கு` `புல_விவரம்` `செய்தியைப்_படி` `செய்தியை_ஆக்கு` `இலக்கங்களா` `பதில்_விளக்கம்` `ஏற்கப்பட்டதா` |
| `attY_cOqaZY.qmz` | 37 tests, bitmaps worked by hand | — |

## uqavi — உதவி (worked examples)

```bash
bash scripts/run_samples.sh attY
```

`ceyqiyY_Akku.qmz` builds and reads back a 0200; `ERkappattaqA.qmz` is the
approval question. Unlike the older packages this is not yet one per function.
