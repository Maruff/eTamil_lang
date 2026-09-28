# qittam — திட்டம்

Project management: the plan, the network, and what the plan is costing.

The work breakdown structure and the hundred-per-cent rule; the critical path, both as plain predecessors and with the four dependency types and lag; crashing, which buys time with money; control accounts and the budget baseline; and earned value, which compares what was planned, what was earned and what was spent.

`Ittu_maqippu.qmz` gives three ways to estimate the cost at completion, because they disagree and the disagreement is the information.

The second half is what a PMO application runs on every day: tasks and their roll-up, the network laid onto a working calendar, the RAG status with the reasons behind it, the change-request lifecycle and the weekly status report. Every lifecycle — a task, a change request — is a `nilYmARRam.qmz` machine held as data, so allowing or refusing a move, stamping who made it and writing the audit event is one call rather than a ladder of conditions per register. Azure DevOps integration and product management build on these and live in `../oruwkiNYppu` and `../qayArippu`.

## Files

| File | What it holds | Functions |
|---|---|---|
| `Ittu_maqippu.qmz` | ஈட்டு மதிப்பு (earned value management) | — |
| `Ittu_vERupAtu.qmz` | ஈட்டு வேறுபாடு: variances and performance indices | `செலவு_வேறுபாடு` `கால_வேறுபாடு` `கணக்கியல்_வேறுபாடு` `செலவுச்_செயல்திறன்` `காலச்_செயல்திறன்` `எதிர்பார்க்கும்_காலம்` |
| `attavaNY.qmz` | அட்டவணை: the network on the calendar | `மணியிலிருந்து_நாட்கள்` `மீதி_நாட்கள்` `நாட்காட்டியில்_இடு` |
| `curukkal.qmz` | சுருக்கல் (crashing, the time-cost trade-off) | `செயல்_நேர_செலவு` `அதிகபட்ச_சுருக்கம்` `செலவுச்_சரிவு` `மலிவான_வேட்பாளர்` `கடுமையானதா` `கூட்டுச்_சரிவு` `இயல்பு_நேரடிச்_செலவு` `மொத்தச்_செலவு` `தாமதச்_செலவுடன்` |
| `kattuppAtu.qmz` | கட்டுப்பாட்டுக் கணக்கு (control accounts, | `கட்டுப்பாட்டுக்_கணக்கு_ஆக்கு` `ஒரே_கணக்கா` `பாதையில்_கணக்குகள்` `கட்டமைப்பை_சரிபார்` `காலப்_பகிர்வு` `திரட்டிய_நிதி` `மொத்த_நிதி` `நிதித்_தொகுதி_ஆக்கு` `அளவீட்டு_அடிப்படை` `அனுமதித்த_நிதி` `இருப்பை_விடுவி` |
| `mARRak_kOrikkY.qmz` | மாற்றக் கோரிக்கை: change control and the scope gate | `மாற்றக்_கோரிக்கை_வரைவு` `புதிய_பணிக்கு_அனுமதியா` |
| `muZZERRa_muRY.qmz` | முன்னேற்ற முறை: how much of a package counts as done | `முறை_0_100` `முறை_50_50` `முறை_மைல்கற்கள்` `ஈட்டிய_மதிப்பு` `நிறைவுப்_பங்கு_நிதியால்` |
| `muZZurYppu.qmz` | முன்னுரைப்பு: what this will cost by the end | `முடிவில்_மதிப்பீடு_மீதியால்` `முடிவில்_மதிப்பீடு_திறனால்` `முடிவில்_மதிப்பீடு_இரண்டாலும்` `முடிக்க_மீதி` `முடிவில்_வேறுபாடு` `முடிக்கத்_தேவையான_திறன்` |
| `nalam.qmz` | நலம்: milestones and the RAG status, with its reasons | `மைல்கல்_ஆக்கு` `மைல்கல்_வேறுபாடு` `கழிந்த_சதவீதம்` `பின்னடைவு_எல்லை` `திட்ட_நலம்` |
| `nilavaram.qmz` | நிலவரம்: the whole picture, as a report block | `நிலவரம்` `நிலவரத்தை_அச்சிடு` |
| `nilY_aRikkY.qmz` | நிலை அறிக்கை: the status report's period and candidates | `அடுத்தவைக்_காலம்` `அறிக்கைக்_காலம்` `அறிக்கை_வேட்பாளர்கள்` |
| `nilYmARRam.qmz` | நிலை மாற்றம்: a lifecycle as data | `நிலை_மாற்றம்_ஆக்கு` `நிலை_வரைவு_ஆக்கு` `அடுத்த_நிலைகள்` `மாற்றம்_அனுமதியா` `இறுதி_நிலையா` `நிலையை_நகர்த்து` |
| `oppanqa_vakY.qmz` | ஒப்பந்த வகை, and who carries the cost risk | `விற்பவர்_இடர்` `உறுதி_நிலை_லாபம்` `ஊக்கக்_கட்டணம்` `கட்டணத்தை_வரம்பிடு` `முழுப்_பொறுப்புப்_புள்ளி` `புள்ளியைத்_தாண்டியதா` `நிலை_விலை_ஊக்கம்` `செலவுடன்_நிலைக்_கட்டணம்` `செலவுடன்_ஊக்கம்` `நேரமும்_பொருளும்` `மிகைச்_செலவின்_விளைவு` |
| `paNi.qmz` | பணி: status, priority, baseline lock, roll-up | `பணி_ஆக்கு` `பணி_நிலைகள்` `பணி_முடிந்ததா` `பணி_நிலுவையா` `முன்னுரிமை_தரம்` `நிலைக்கான_நிறைவு` `பூட்டிலும்_மாறுபவை` `பூட்டிய_மாற்றம்_சரியா` `திரட்டிய_நிறைவு` `எடையிட்ட_நிறைவு` `பெற்றோர்_முன்_வரிசை` |
| `paNi_cOqaZY.qmz` | tests for the task, schedule, health and change-control modules | — |
| `pAqY.qmz` | கடுமையான பாதை (the critical path method) | `செயல்_ஆக்கு` `செயலைத்_தேடு` `வழித்தோன்றல்கள்` `முன்னோடிகள்_உள்ளனவா` `சுழற்சி_உள்ளதா` `வலையைக்_கணக்கிடு` `கடுமையானவை` `உணர்திறன்_வலையா` `பெறு_புலம்` `குறி_உள்ளதா` `பதிவைத்_தேடு` `பதிவை_மாற்று` |
| `pakuppu.qmz` | வேலைப் பகுப்பு (the work breakdown structure) | `கணு_ஆக்கு` `கணு_தேடு` `சேய்கள்` `மூலங்கள்` `இலைக்_கணுவா` `வேலைத்_தொகுப்புகள்` `உள்_கூட்டல்` `ஆழம்` `நூறு_விதி_சரியா` `விதியை_சரிபார்` |
| `qittam_cOqaZY.qmz` | tests for the project costing modules | `காலத்தைப்_பெறு` |
| `qotarpu.qmz` | தொடர்பு, the four dependency types and lag | `தொடர்பு_சுற்று_எல்லை` `முனை_ஆக்கு` `தொடர்பு_ஆக்கு` `தொடர்பு_வகை_சரியா` `முன்னோட்டமா` `முனையைப்_பெறு` `கால_அளவைப்_பெறு` `முந்தியைப்_பெறு` `முந்தியை_அமை` `தாழ்த்தியை_அமை` `தேவையான_தொடக்கம்` `அனுமதித்த_முடிவு` `தொடர்பு_வலையைக்_கணக்கிடு` `தொடர்பு_கடுமையானவை` `தொடர்பு_முனையின்_புலம்` `மொத்தத்_தாமதம்` `முன்னோட்டங்கள்` `தாமதப்_பங்கு` `சுருக்கக்கூடிய_காலம்` |

## uqavi — உதவி (worked examples)

`uqavi/` holds 121 runnable programs, one for each function above, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh qittam
```
