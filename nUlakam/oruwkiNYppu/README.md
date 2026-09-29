# oruwkiNYppu — ஒருங்கிணைப்பு

Integration with Azure DevOps: keeping a PMO's tasks and a team's work items the same work, without either side re-keying it and without the two fighting over who changed what last.

Most of a two-way sync is not the HTTP call. It is deciding which status is which state when every process names its states differently (`varYpatam.qmz` falls back on the five fixed state categories); refusing, visibly, writes to fields the other side does not own (`pula_urimY.qmz`); not applying our own change when it comes back as a service hook (`varavu.qmz`); sending parents before children and retrying the right failures the right way (`aZuppu_varicY.qmz`, `mItci.qmz`); and not overwriting a teammate's edit made a second earlier (the `/rev` test in `ottu.qmz`). The REST client in `ajUr.qmz` is the small part, and it gives every failure one shape so the outbox handles them all the same way.

Task statuses and priorities are the ones in `../qittam/paNi.qmz`. Nothing here reads the clock or makes up a random number: the current time and the retry jitter are arguments, so every decision can be replayed in a test.

## Files

| File | What it holds | Functions |
|---|---|---|
| `aZuppu_varicY.qmz` | அனுப்பு வரிசை: the outbox | `அனுப்பு_உருப்படி_ஆக்கு` `வரிசையில்_உள்ளதா` `அனுப்பத்_தயாரானவை` `அனுப்பியதாகப்_பதி` `தோல்வியைப்_பதி` |
| `ajUr.qmz` | அஜூர்: the Azure DevOps REST client | `அஜூர்_பதிப்பு` `அஜூர்_இணைப்பு_ஆக்கு` `அஜூர்_தலைப்புகள்` `திட்ட_உரலி` `பணியுருப்படி_உரலி` `அஜூர்_அனுப்பு` `பணியுருப்படியைப்_பெறு` `பணியுருப்படியை_உருவாக்கு` `பணியுருப்படியைத்_திருத்து` `வினாவை_ஓட்டு` |
| `kYrEkY.qmz` | கைரேகை: fingerprints and idempotency keys | `நிலையான_வடிவம்` `கைரேகை` `ஒருமுறைக்_குறி` `கைரேகை_ஒன்றா` |
| `mItci.qmz` | மீட்சி: retrying a call that failed | `அதிக_முயற்சிகள்` `மேல்_காத்திருப்பு` `மீள்_முயலத்தக்கதா` `காத்திருப்பு_நொடிகள்` `கைவிடலாமா` |
| `oruwkiNYppu_cOqaZY.qmz` | tests for the integration modules | — |
| `ottu.qmz` | ஒட்டு: JSON Patch bodies for work items | `ஒட்டுச்_செயல்` `புல_ஒட்டு` `திருத்தச்_சோதனை` `பெற்றோர்_ஒட்டு` `குறிச்சொற்களை_இணை` `பணி_ஒட்டுகள்` `ஒட்டுகள்_சரியா` |
| `pula_urimY.qmz` | புல உரிமை: which side owns each field | `புல_உரிமை_ஆக்கு` `இயல்பு_உரிமைகள்` `உரிமையாளர்` `வரவைப்_பிரி` `அனுப்பவேண்டியவை` |
| `varavu.qmz` | வரவு: service hooks and echoes | `வரவு_முடிவு` `கொக்கிச்_சீட்டு_சரியா` `கொக்கி_நிகழ்வைப்_படி` |
| `varYpatam.qmz` | வரைபடம்: statuses, priorities, types, progress | `வெளி_நிலை_வேட்பாளர்கள்` `நிலையின்_வகைமை` `வெளி_நிலை` `உள்_நிலை` `முன்னுரிமை_எண்` `எண்ணிலிருந்து_முன்னுரிமை` `வகை_விதி_ஆக்கு` `பணியுருப்படி_வகை` `மணியிலிருந்து_நிறைவு` |
| `viZA.qmz` | வினா: WIQL for the reconciliation sweep | `தொடக்கத்_தேடல்_நாட்கள்` `வினா_மேற்கோள்` `தேடல்_தொடக்க_நாள்` `மாறியவை_வினா` |

## uqavi — உதவி (worked examples)

`uqavi/` holds one runnable program for each function above, named
after the function it demonstrates. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.
The ones that send a request point at `127.0.0.1:9`, which refuses at once, so
they run offline and show the failure shape a caller branches on.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh oruwkiNYppu
```
