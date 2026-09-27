# vaLam — வளம்

Resources, what they cost, and how that cost reaches the ledger.

A resource has a rate that changes over time, so `விகிதத்தைத்_தேடு` asks which rate was in force on a date rather than which is current. Timesheets carry a state, and only approved entries are costed.

`paqivu.qmz` posts the result: labour, materials, subcontract, overhead, revenue, invoicing, advances and the provision for a contract that will lose money.

## Files

| File | What it holds | Functions |
|---|---|---|
| `nEraqqAL.qmz` | நேரத்தாள், the timesheet | `நேர_உள்ளீடு` `உள்ளீட்டைச்_சரிபார்` `நாள்_மொத்த_மணிகள்` `மிகைப்_பதிவா` `நிலை_மாற்றம்_சரியா` `நிலையை_மாற்று` `ஏற்றவை_மட்டும்` `உள்ளீட்டைக்_கணக்கிடு` `தாளைக்_கணக்கிடு` `பணிக்கு_மொத்தம்` `கட்டண_மணிகள்` `வளத்துக்கு_மொத்தம்` |
| `paqivu.qmz` | பதிவு, project cost into the ledger | `நடப்புக்_கணக்குக்_குறி` `உழைப்புப்_பதிவு` `பொருள்_பதிவு` `துணை_ஒப்பந்தப்_பதிவு` `மேல்நிலைப்_பதிவு` `வருவாய்ப்_பதிவு` `விலைப்பட்டியல்_பதிவு` `முன்பணப்_பதிவு` `நட்ட_ஒதுக்கீட்டுப்_பதிவு` `ஈட்டிய_செலவு` `காலம்_வரை_செலவு` `செலவை_ஒப்பிடு` |
| `vaLam.qmz` | வளம், resources and what they cost | `வளம்_ஆக்கு` `விகிதப்_பதிவு` `விகிதத்தைத்_தேடு` `மேலதிக_விகிதம்` `காலத்_திறன்` `கிடைக்கும்_மணிகள்` `பயன்பாட்டு_விகிதம்` `கட்டண_விகிதம்` `வளத்தைத்_தேடு` |
| `vaLam_cOqaZY.qmz` | tests for resources, timesheets and posting | — |
