# nUlakam — the eTamil standard library

Written in eTamil, not Rust. That is the point: if the standard library
needed a systems language, the DSL would not be sufficient for the
frameworks built on top of it.

```etamil
இறக்கு "nUlakam/paNam.qmz";

அச்சு ரூபாய்(12345678.5);        // ₹1,23,45,678.50
```

## Modules

| File | Contents |
|---|---|
| `col.qmz` | strings — `துண்டு` `தேடு` `ஒழுங்கு` `தொடங்குகிறதா` `முடிகிறதா` `திரும்பச்செய்` `இடமிருந்து_நிரப்பு` |
| `kaNiqam.qmz` | math — `முழுமதிப்பு` `சிறியது` `பெரியது` `கூட்டு` `சராசரி` `சதவீதம்` `வர்க்கமூலம்` |
| `aNi.qmz` | arrays — `உள்ளதா` `இடம்_காண்` `தலைகீழ்` `வெட்டு` `புலம்_எடு` `காலியா` |
| `poruL.qmz` | records — `புலம்_உள்ளதா` `புலம்_அல்லது` `புலங்கள்` `மதிப்பீடுகள்` `காலியா_பதிவேடு` |
| `cOqaZY.qmz` | tests — `சோதனை_தொடக்கம்` `உறுதிசெய்` `சமம்` `வேறுபடு` `சேர்_ஓட்டம்` `சோதனை_முடிவு` |
| `kOppumuRY.qmz` | the file system — `பாதை_இணை` `அடிப்பெயர்` `கோப்பக_பெயர்` `நீட்சி` `பெயர்_மட்டும்` `பாதை_இயல்பாக்கு` `கோப்பகமா` `கோப்பக_பட்டியல்` `கோப்பக_நட` `நீட்சியால்_கோப்புகள்` `வினாடியை_நாளாக` |
| `vaZporuL/vaZporuL.qmz` | hardware, one API for Raspberry Pi, the simulated board and (through artino) Arduino — `பலகை` `முனை_வெளியீடு` `முனை_எழுது` `முனை_படி` `முனை_மாற்று` `ஒப்புமை_படி` `மில்லி_நொடி` `காத்திரு` `தொடர்_திற` `தொடர்_வரி_படி` `தொடர்_வரி_எழுது` `தொடர்_மூடு` `ஒலி_எழுப்பு` `ஒலி_நிறுத்து` `காவல்_தொடங்கு` `காவல்_புதுப்பி`, and `போலி_*` for tests. On the VM: `ETAMIL_BOARD=sim` anywhere, a Raspberry Pi's GPIO, serial ports on Linux and macOS |
| `vaZporuL/yUnO.qmz` `nAnO.qmz` `mekA.qmz` `pIkO.qmz` `rAspY.qmz` | one board each — Uno, Nano, Mega, Pico and Pico 2, Raspberry Pi: the hardware API plus the board's pin names under the same names everywhere, `விளக்கு_முனை` `ஒப்புமை_0`… `ஒப்புமை_மில்லிவோல்ட்` `USB_துறை` `தொடர்1_துறை`…; the Pi's `தலைப்பு_முனை` maps header pins to BCM. artino refuses a board file that does not match `--board` |
| `vawki/vatti.qmz` | interest — `எளிய_வட்டி` `நாளாந்த_வட்டி` `கூட்டு_வட்டி` `முதிர்வுத்_தொகை` `அடுக்கு` `நாட்கள்` |
| `vawki/kataZ.qmz` | loans — `மாதத்_தவணை` `முன்வரித்_தவணைகள்` `மொத்த_வட்டி` `மொத்தத்_திருப்பி` `முன்கூட்டியே_அடைத்தால்` |
| `vawki/coqqu.qmz` | asset classification — `விதிமுறைகளை_ஏற்று` `வகைப்படுத்து` `ஒதுக்கீடு` `சரிபார்க்கப்படாதவை` |
| `cawkili/fabric.qmz` | Hyperledger Fabric — `நுழைவு` `மதிப்பிடு` `சமர்ப்பி` `மீண்டும்_சமர்ப்பி` `மோதலா` |
| `upi/vilAcam.qmz` | UPI addresses and pay links — `முகவரி_சரியா` `தொகை_சரியா` `தொகை_உரை` `பணம்_இணைப்பு` `இணைப்பைப்_படி` |
| `upi/nilYmY.qmz` | UPI payment states — `பணம்_வந்ததா` `சரிபார்க்கவா` `நகர்வு_சரியா` `நகர்த்து` |
| `nAtkAtti/nAL.qmz` | dates — `நாள்_செல்லுபடியா` `நாள்_ஆக்கு` `நெட்டாண்டா` `மாத_நாட்கள்` `மாத_இறுதி` `மாதங்களைக்_கூட்டு` `ஆண்டுகளைக்_கூட்டு` `முடிந்த_மாதங்கள்` `பகுதி_மாதங்கள்` `வாரநாள்` `நிதியாண்டின்_தொடக்க_ஆண்டு` `கால்_ஆண்டு_எண்` |
| `nAtkAtti/vElYnAL.qmz` | working days — `நாட்காட்டி_ஆக்கு` `வேலை_நாளா` `அடுத்த_வேலை_நாள்` `நகர்த்தி_வேலை_நாள்` `வேலை_நாட்களைக்_கூட்டு` `வேலை_நாட்களை_எண்ணு` `முடிவு_நாள்` `பணி_அட்டவணை` `உருட்டு` |
| `kaNakkiyal/kaNakkukaL.qmz` | chart of accounts — `வகை_சொத்து` `வகை_பொறுப்பு` `கணக்கு_ஆக்கு` `செல்லுபடியா` `பற்று_இயல்பா` `கணக்கு_தேடு` `வகையால்_வடிகட்டு` |
| `kaNakkiyal/pErEtu.qmz` | journal and ledger — `பற்று_வரிசை` `வரவு_வரிசை` `பரிவர்த்தனை_ஆக்கு` `சமநிலையா` `பதிவிடு` `கணக்கு_இருப்பு` `காலம்_வடிகட்டு` |
| `kaNakkiyal/vari.qmz` | GST entries — `வரி_தொகை` `அடிப்படையை_பிரி` `மாநில_பிரிப்பு` `விற்பனை_பரிவர்த்தனை` `கொள்முதல்_பரிவர்த்தனை` `வரவு_குறிப்பு` `எதிர்_பதிவு` |
| `kaNakkiyal/kAlam.qmz` | accounting periods — `காலம்_ஆக்கு` `இந்திய_ஆண்டு` `நாட்காட்டி_ஆண்டு` `காலத்தில்_உள்ளதா` `வரையிலா` |
| `kaNakkiyal/oqukkItu.qmz` | allocation and ageing — `ஒதுக்கீடு_ஆக்கு` `பயன்படுத்தப்பட்டது` `நிலுவைத்_தொகை` `ஒதுக்கு` `வயது_அட்டவணை` |
| `kaNakkiyal/aRikkYkaL.qmz` | the statements — `இருப்பாய்வு` `வருமான_அறிக்கை` `இருப்புநிலை` `பணப்புழக்க_அறிக்கை` `கணக்கு_அறிக்கை` |
| `kaNakkiyal/niRuvaZam.qmz` | entities and currency — `நிறுவனம்_ஆக்கு` `நிறுவன_வடிகட்டு` `மாற்று_விகிதம்_ஆக்கு` `அடிப்படைக்கு_மாற்று` `அன்னிய_வேறுபாடு` |
| `kaNakkiyal/mutippu.qmz` | period close — `முடிப்பு_பரிவர்த்தனை` `ஆண்டை_முடி` `முடிக்கப்பட்டதா` |
| `kaNakkiyal/qEymAZam.qmz` | depreciation — `நேர்கோட்டு_ஆண்டு` `குறையும்_ஆண்டு` `பகுதி_ஆண்டு` `நேர்கோட்டு_அட்டவணை` `குறையும்_அட்டவணை` `தொகுதி_தேய்வு` |
| `kaNakkiyal/Uqiyam.qmz` | payroll — `மொத்தச்_சம்பளம்` `நாட்களுக்கு_ஏற்ப` `வரம்புடன்_பங்களிப்பு` `தகுதிக்குள்_பங்களிப்பு` `படிநிலை_வரி` `பணிக்கொடை` `சம்பளச்_சீட்டு` |
| `kaNakkiyal/vari_vikiqam.qmz` | tax rates — `விகிதம்_தேடு` `படிகளை_ஏற்று` `படி_வரி_கணக்கிடு` `உள்_மாநிலமா` `மாநிலப்_பெயர்` |
| `celavu/nilYyam.qmz` | cost centres, objects and elements — `நிலையம்_ஆக்கு` `உற்பத்தி_நிலையமா` `வகையால்_நிலையங்கள்` `செலவுப்_பொருள்_ஆக்கு` `கூறு_ஆக்கு` `மாறும்_பகுதி` `நிலையான_பகுதி` |
| `celavu/celavu_qAL.qmz` | the cost sheet — `நுகர்ந்த_பொருட்கள்` `முதன்மைச்_செலவு` `தொழிற்சாலைச்_செலவு` `உற்பத்திச்_செலவு` `விற்பனைச்_செலவு` `அலகுக்குச்_செலவு` `செலவுத்_தாள்_ஆக்கு` |
| `celavu/mElnilY.qmz` | overhead absorption — `உள்வாங்கல்_விகிதம்` `உள்வாங்கியது` `உள்வாங்கல்_வேறுபாடு` `அதிக_உள்வாங்கலா` `நிலையங்களுக்குப்_பகிர்` `சேவையை_மறுபகிர்` |
| `celavu/niyamam.qmz` | standard costing variances — `பொருள்_வீத_வேறுபாடு` `பொருள்_பயன்பாட்டு_வேறுபாடு` `ஊதிய_வீத_வேறுபாடு` `ஊதிய_திறன்_வேறுபாடு` `விற்பனை_அளவு_வேறுபாடு` `சாதகமா` `வேறுபாட்டு_உரை` |
| `celavu/pawkaLippu.qmz` | marginal costing and CVP — `அலகு_பங்களிப்பு` `லாபம்` `பங்களிப்பு_விகிதம்` `சமநிலை_அலகுகள்` `பாதுகாப்பு_வரம்பு` `இலக்கு_அலகுகள்` `அலகு_வளத்திற்கு_பங்களிப்பு` |
| `celavu/mILpakirvu.qmz` | reciprocal service apportionment — `சேவை_நிலையம்_ஆக்கு` `இரு_சேவை_ஒரேசமயம்` `மீண்டும்_பகிர்` `படிநிலைப்_பகிர்வு` `படிநிலை_வரிசை_வேறுபாடு` |
| `celavu/ceyalmuRY.qmz` | process costing and equivalent units — `இயல்பு_இழப்பு_அலகுகள்` `அலகுக்கான_செலவு` `அசாதாரண_இழப்பின்_மதிப்பு` `செயல்முறையைக்_கணக்கிடு` `சராசரி_சமமான_அலகுகள்` `fifo_சமமான_அலகுகள்` |
| `celavu/ceyalpAtu.qmz` | activity-based costing — `செயல்பாடு_ஆக்கு` `இயக்கி_விகிதம்` `தயாரிப்பின்_மேல்நிலை` `குறுக்கு_மானியம்` `பயனளிக்குமா` |
| `qittam/pakuppu.qmz` | the work breakdown structure — `கணு_ஆக்கு` `சேய்கள்` `இலைக்_கணுவா` `வேலைத்_தொகுப்புகள்` `உள்_கூட்டல்` `ஆழம்` `நூறு_விதி_சரியா` `விதியை_சரிபார்` |
| `qittam/kattuppAtu.qmz` | control accounts, time-phased budget and reserves — `கட்டுப்பாட்டுக்_கணக்கு_ஆக்கு` `கட்டமைப்பை_சரிபார்` `திரட்டிய_நிதி` `அளவீட்டு_அடிப்படை` `அனுமதித்த_நிதி` `இருப்பை_விடுவி` |
| `qittam/Ittu_maqippu.qmz` | earned value — `முறை_0_100` `முறை_50_50` `முறை_மைல்கற்கள்` `ஈட்டிய_மதிப்பு` `செலவு_வேறுபாடு` `கால_வேறுபாடு` `செலவுச்_செயல்திறன்` `முடிக்கத்_தேவையான_திறன்` `நிலவரம்` |
| `qittam/pAqY.qmz` | the critical path — `செயல்_ஆக்கு` `வலையைக்_கணக்கிடு` `கடுமையானவை` `பெறு_புலம்` `சுழற்சி_உள்ளதா` `உணர்திறன்_வலையா` |
| `qittam/curukkal.qmz` | crashing — `செயல்_நேர_செலவு` `செலவுச்_சரிவு` `அதிகபட்ச_சுருக்கம்` `மலிவான_வேட்பாளர்` `கூட்டுச்_சரிவு` `மொத்தச்_செலவு` `தாமதச்_செலவுடன்` |
| `itar/itar.qmz` | the risk register — `இடர்_ஆக்கு` `எதிர்பார்ப்பு` `மொத்த_எதிர்பார்ப்பு` `இடர்_மதிப்பெண்` `பதிலளிப்பு_சரியா` `எஞ்சிய_இடர்` `பதிலளிப்பு_பயனுள்ளதா` `தற்செயல்_இருப்பைக்_கணக்கிடு` `மேலாண்மை_இருப்பைக்_கணக்கிடு` `கிளைகளின்_எதிர்பார்ப்பு` `சிறந்த_முடிவு` |
| `itar/mUZRuppuLLi.qmz` | three-point estimation — `முப்புள்ளி_ஆக்கு` `பீட்டா_சராசரி` `முக்கோண_சராசரி` `நியம_விலகல்` `பரவல்` `பாதை_சராசரி` `பாதை_பரவல்` `பாதை_விலகல்` `_நிகழ்தகவு_Z` `இலக்கு_நிகழ்தகவு` `நம்பிக்கை_காலம்` |
| `vaLam/vaLam.qmz` | resources and effective-dated rates — `வளம்_ஆக்கு` `விகிதப்_பதிவு` `விகிதத்தைத்_தேடு` `மேலதிக_விகிதம்` `காலத்_திறன்` `கிடைக்கும்_மணிகள்` `பயன்பாட்டு_விகிதம்` `கட்டண_விகிதம்` |
| `vaLam/nEraqqAL.qmz` | timesheets — `நேர_உள்ளீடு` `உள்ளீட்டைச்_சரிபார்` `மிகைப்_பதிவா` `நிலை_மாற்றம்_சரியா` `ஏற்றவை_மட்டும்` `தாளைக்_கணக்கிடு` `பணிக்கு_மொத்தம்` `கட்டண_மணிகள்` |
| `vaLam/paqivu.qmz` | project cost into the ledger — `நடப்புக்_கணக்குக்_குறி` `உழைப்புப்_பதிவு` `பொருள்_பதிவு` `மேல்நிலைப்_பதிவு` `வருவாய்ப்_பதிவு` `விலைப்பட்டியல்_பதிவு` `ஈட்டிய_செலவு` `காலம்_வரை_செலவு` `செலவை_ஒப்பிடு` |
| `varuvAy/oppanqam.qmz` | Ind AS 115 steps 1-4 — `ஒப்பந்தம்_உள்ளதா` `கடமை_ஆக்கு` `தனிக்_கடமையா` `மாறும்_எதிர்பார்ப்பு` `மிக_வாய்ப்பான_தொகை` `கட்டுப்பாட்டுடன்` `பரிமாற்ற_மதிப்பு` `கடமைகளுக்கு_ஒதுக்கு` `எஞ்சிய_முறை` `தள்ளுபடியை_ஒதுக்கு` |
| `varuvAy/muZZERRam.qmz` | step 5, progress and revenue — `காலப்போக்கிலா` `செலவு_முன்னேற்றம்` `வெளியீட்டு_முன்னேற்றம்` `நிறுவாத_பொருளுடன்_வருவாய்` `இதுவரை_வருவாய்` `இந்தக்_காலத்து_வருவாய்` `மீட்பளவு_வருவாய்` `வருவாய்_நிலவரம்` |
| `varuvAy/mARRam.qmz` | contract modifications — `மாற்றம்_ஆக்கு` `தனி_ஒப்பந்தமா` `மாற்ற_வகை` `எஞ்சிய_மதிப்பு` `எஞ்சிய_அலகு_மதிப்பு` `பிடிப்புத்_தொகை` `மூன்று_வழியும்` `மாற்றத்தைப்_பயன்படுத்து` |
| `varuvAy/mukavar.qmz` | principal or agent — `குறியீட்டு_எண்ணிக்கை` `முதன்மை_வருவாய்` `முகவர்_வருவாய்` `வருவாயை_அளவிடு` `இரு_வழியும்` |
| `varuvAy/iruppukaL.qmz` | contract assets and liabilities — `ஒப்பந்த_நிலை` `உரிமை_வகை` `தேக்கத்_தொகை` `விலைப்பட்டியல்_நிகரம்` `ஒப்பந்தங்களைத்_தொகு` `தவறாக_நிகரமிட்டால்` |
| `varuvAy/oppanqac_celavu.qmz` | contract costs, and onerous contracts under Ind AS 37 — `பெறுதல்_செலவின்_வகை` `நிறைவேற்றல்_செலவின்_வகை` `கழிப்புத்_தொகை` `முதலீட்டுக்_குறைவு` `தவிர்க்க_முடியாத_செலவு` `நட்டம்_தருமா` `நட்டக்_கணக்கு` |
| `nErativari/varumAZam.qmz` | the five heads and loss set-off — `சம்பள_வருமானம்` `நிகர_ஆண்டு_மதிப்பீடு` `வாடகைச்_சொத்து_வருமானம்` `சொந்த_வீட்டு_வருமானம்` `வீட்டு_இழப்பை_ஈடுசெய்` `குறியீட்டுச்_செலவு` `மூலதன_ஆதாயம்` `ஈடுசெய்ய_முடியுமா` `ஈட்டைச்_செய்` `மொத்த_வருமானம்` |
| `nErativari/kazivukaL.qmz` | Chapter VI-A deductions — `கழிவு_ஆக்கு` `தனி_வரம்பு` `கூட்டுக்_கழிவுகள்` `மீதமுள்ள_இடம்` `நன்கொடைக்_கழிவு` `அனுமதிக்கத்தக்க_கழிவு` `வருமானத்தை_வட்டமிடு` `மொத்த_வரிக்குரிய_வருமானம்` |
| `nErativari/varikkaNakku.qmz` | total income to tax payable — `அடிப்படை_வரி` `தள்ளுபடியைக்_கணக்கிடு` `மேல்வரி_தொகை` `விளிம்பு_நிவாரணம்` `கழிவு_வரி` `வரியை_வட்டமிடு` `வரியைக்_கணக்கிடு` `முறைகளை_ஒப்பிடு` |
| `nErativari/mUlavari.qmz` | tax deducted at source — `பிரிவு_ஆக்கு` `பிடிக்க_வேண்டுமா` `பிடித்த_அடிப்படை` `பயன்படும்_வீதம்` `பிடித்தத்_தொகை` `ஆண்டுப்_பிடித்தம்` `பிடிக்காத_வட்டி` `சம்பள_பிடித்தம்` |
| `nErativari/muZvari.qmz` | advance tax and interest — `மதிப்பிட்ட_வரி` `மூத்தவர்_விலக்கு` `முன்வரித்_தவணைகள்` `தவணை_அட்டவணையை_ஆக்கு` `வட்டிக்குரிய_அடிப்படை` `தவணை_வட்டி` `குறைபாட்டு_வட்டி` `தாமதத்_தாக்கல்_வட்டி` `செலுத்த_வேண்டியது` |
| `nErativari/oqqivari.qmz` | deferred tax under Ind AS 12 — `தற்காலிக_வேறுபாடு` `வேறுபாட்டு_வகை` `ஒத்திவரிக்_கணக்கு` `அங்கீகரிக்கத்தக்க_சொத்து` `இழப்பின்_ஒத்திவரி` `ஈடுசெய்யலாமா` `ஒத்திவரி_மாற்றம்` `வீத_ஒப்புரவு` |
| `qittam/qotarpu.qmz` | the four dependency types and lag — `முனை_ஆக்கு` `தொடர்பு_ஆக்கு` `தொடர்பு_வலையைக்_கணக்கிடு` `தொடர்பு_கடுமையானவை` `மொத்தத்_தாமதம்` `முன்னோட்டங்கள்` `தாமதப்_பங்கு` `சுருக்கக்கூடிய_காலம்` |
| `qittam/oppanqa_vakY.qmz` | contract types — `விற்பவர்_இடர்` `உறுதி_நிலை_லாபம்` `ஊக்கக்_கட்டணம்` `முழுப்_பொறுப்புப்_புள்ளி` `நிலை_விலை_ஊக்கம்` `செலவுடன்_நிலைக்_கட்டணம்` `செலவுடன்_ஊக்கம்` `நேரமும்_பொருளும்` `மிகைச்_செலவின்_விளைவு` |
| `kAppItu/kAppItu.qmz` | insurance — `முனைமம்` `ஆயிரத்திற்கு_முனைமம்` `சராசரி_விதி` `கோரல்_தீர்வு` `கோரல்_இல்லா_சலுகை` `நிலுவைக்_கோரல்கள்` |
| `cuwkam/cuwkam.qmz` | customs and trade — `மதிப்பிடத்தக்க_மதிப்பு` `சுங்கக்_கணக்கு` `தலைப்பு_சரியா` `பொருந்துமா` `செல்லுபடி_நாட்கள்` `வழிச்சீட்டு_சரிபார்` |
| `qaLam/retis.qmz` | Redis — `சேமி` `காலத்துடன்_சேமி` `எடு` `இருக்கிறதா` `நீக்கு` `ஒன்று_கூட்டு` `முன்_சேர்` `வரிசைப்_பகுதி` `இல்லையெனில்_இயல்பு` |
| `paNam.qmz` | money — `ரூபாய்` `காசு_வடிவம்` `காசாக` `லட்சம்` `கோடி` |
| `kAcu.qmz` | paise-exact money — `ரூபாயும்_பைசாவும்` `ரூபாயாக` `காசு_உரை` `காசு_கூட்டு` `விழுக்காடு_காசு` `சமமாகப்_பிரி` `விகிதத்தில்_பிரி` |
| `jEcAZ.qmz` | JSON — `ஜேசான்_ஆக்கு` `ஜேசான்_படி` |
| `kuRiyAkkam.qmz` | encoding — `அறுபத்துநான்கு_ஆக்கு` `அறுபத்துநான்கு_படி` `பதினாறு_ஆக்கு` `பதினாறு_படி` |
| `AvaNam.qmz` | documents — `ஆவணம்_நிரப்பு` `பொதியை_நிரப்பு` `_pdf_ஆக்கு`, and the `_ODT_வடிவம்` / `_ODS_வடிவம்` / `_DOCX_வடிவம்` / `_XLSX_வடிவம்` shapes |

## JSON is written here, not in the host

A parser needs to build a record whose field names come from the data, and the
VM already allows that: `பொருள்[சாவி] = மதிப்பு` computes the key at runtime.
So `jEcAZ.qmz` is ordinary eTamil, and Layer 0 gains nothing.

```etamil
இறக்கு "nUlakam/jEcAZ.qmz";

ப = மதிப்பு(ஜேசான்_படி(request_body));
அச்சு ப["qokY"] + 1;                       // a number, not text
பதில் 200, ஜேசான்_ஆக்கு({நிலுவை: 1500});
```

`ஜேசான்_படி` returns `சரி`/`தவறு`, so malformed input is handled rather than
guessed at. Record fields serialize in sorted order, which makes a response
body stable enough to assert on. `\uXXXX` escapes are not decoded.

## What the host provides

Only what cannot be expressed in the language itself:

`நீளம்` `இணை` `வகை` · `சரி` `தவறு` `சரியா` `தவறா` `மதிப்பு` `இயல்பு` ·
`வட்டமிடு` `தரை` `மேல்` · `சொல்லாக்கு` `எண்ணாக்கு` · `மேல்_எழுத்து` `கீழ்_எழுத்து` ·
`இன்று` `நாள்_வேறுபாடு` `நாள்_கூட்டு` ·
`கடவுச்சொல்_மறை` `கடவுச்சொல்_சரியா` `சீட்டு_ஆக்கு` `சீட்டு_சரிபார்` ·
`கையொப்பம்` `கையொப்பம்_சரியா` · `வலை_பெறு` `வலை_பதி` `வலை_அனுப்பு` ·
`பைட்டுகள்` `பைட்டுச்_சரம்`

The last nine are bcrypt, JWT, HMAC and HTTP: hashing and signing need bytes,
randomness and a constant-time comparison the language cannot reach, and opening
a socket is a syscall. The last two are the only thing base64 needed: a byte is
not something the language can reach on its own, but once it has an array of
them the encoding is arithmetic, so `kuRiyAkkam.qmz` is ordinary eTamil. Everything above them — who a user is,
which route needs which role — stays in eTamil, and a token's payload crosses
the boundary as JSON text that `jEcAZ.qmz` reads and writes.

Everything in this directory is built from those. Each also answers to a
romanized name, and to an English one written with a leading underscore:
`நீளம்` is also `nILam` and `_length`, `இணை` is also `iNY` and `_append`.
The underscore is part of the name — `length` on its own is not a builtin, it
is a name your program is free to use.

`docs/reference/KEYWORDS.md` is the reference for the language's *keywords*,
which are a different set: none of the builtins above appear in it. The three
forms of each are in `call_builtin` in `etamil_compiler/src/vm/interpreter.rs`.

## Two behaviours worth knowing

**Strings are measured in written letters, not code points.**
`நீளம்("வணக்கம்")` is 5. A Tamil letter is often a consonant plus a vowel
sign or pulli, so counting code points would give 7 and every helper here
would be wrong on Tamil text.

**A field name is stored exactly as written**, keyword or not: `{வரி: 1000}`
produces the field `வரி`. It used to be filed under the English token name
`Tax`, which anglicised the author's own words; that changed with roadmap item
2. The consequence is that `{வரி: 1}` and `{vari: 1}` are now *different*
fields, so a program should pick one spelling and keep to it.

## Import paths

`இறக்கு` looks beside the importing file, then along `ETAMIL_PATH`, then next
to the compiler binary. To use the library from anywhere:

```bash
export ETAMIL_PATH=/path/to/etamil_compiler
etamil --vm my_program.qmz
```

## Missing, and why

No `map` or `filter`: the language has no first-class functions yet. When
function values arrive, several loops here collapse to one line.

## பிரி and ஒன்றிணை moved to the host

They were written here, like everything else. Both walked the string one
letter at a time, and every read re-segmented the whole string into written
letters, so splitting a document cost O(n²) segmentations — measured at 14
seconds over 8 KB, and 400 KB never finished. They are host builtins now,
along with `மாற்று`, doing one segmentation pass and then a byte search.

A separator still only matches on a letter boundary, so `பிரி("கா", "ா")`
does not cut a letter in half. The pieces are the same pieces; only the cost
differs. A function defined here would shadow a builtin, so they are gone
from `col.qmz` rather than left to delegate.

## Documents are rendered here, not in the host

`.odt`, `.ods`, `.docx` and `.xlsx` are all zip archives of XML. The host
opens and rewrites the archive — `பொதி_படி` and `பொதி_மாற்று` — because a
picture inside one is not text and could not survive being a `சரம்`. What a
template *means* is decided in `AvaNam.qmz`: which placeholder gets which
value, which rows repeat, what has to be escaped.

```etamil
இறக்கு "nUlakam/AvaNam.qmz";

மதிப்புகள் = [{"குறி": "project.name", "மதிப்பு": "Beak PMO"}];
தொகுதிகள் = [{"பெயர்": "o",
              "புலங்கள்": ["no", "objective"],
              "வரிசைகள்": [{"no": "1", "objective": "One source of truth"}]}];

பொதியை_நிரப்பு("charter.odt", "out.odt", _ODT_வடிவம், மதிப்புகள், தொகுதிகள்);
```

A `வடிவம்` is the whole of the difference between the formats: which entry
holds the text, and what a table row is called there. `{{ name }}` is a value
and `{%tr for x in list %}` … `{%tr endfor %}` repeats the rows between them,
which is the convention the templates already used.

An `.xlsx` keeps its text in a shared table and its rows in the sheet, so a
repeating row there would have to renumber shared-string indexes. Scalars
work; row groups do not, and `_XLSX_வடிவம்` says so rather than half-doing it.

## Tests are written here too

A library written in eTamil used to be testable only from Rust, or by running
an example and seeing whether it exited 0. Neither says which assertion failed.
`cOqaZY.qmz` is the alternative, and `kaNakkiyal/vari_cOqaZY.qmz` is a suite
using it — fifteen assertions about GST arithmetic, runnable like any program:

```bash
etamil --vm nUlakam/kaNakkiyal/vari_cOqaZY.qmz
```

It exits non-zero when anything fails, so a suite that reports a failure also
fails whatever ran it. The run is threaded through each assertion rather than
kept in a module variable, because a function cannot change a global: assigning
to a name inside a `செயல்` makes a local, and a counter incremented there is
lost on return.

## Banking: the engine here, the figures in a table

`vawki/` holds interest, loans and asset classification. Not one regulatory
number is in any of them.

Interest and instalments are arithmetic, so those modules take a rate as an
argument and hold none. `அடுக்கு` raises a base by repeated multiplication
rather than borrowing a floating-point `pow`: every number here is a
fixed-point decimal precisely so that money does not drift, and a float at the
step that compounds would put the drift back.

The amortisation schedule closes exactly. An instalment is rounded to the
paisa and paid two hundred and forty times, so the rounded parts do not sum to
the loan; the last row absorbs the difference. A schedule ending four paise
overdrawn is not a rounding detail, it is an account that will not close.

Classification is different, because the day counts and provisioning
percentages are set by circular and amended. They live in
`vawki/coqqu_viqimuRY.sql`, effective-dated, so that a review run against last
year's rules still produces last year's answer — and so that an amendment is a
new row rather than an edit. **The figures shipped there are placeholders and
are marked as such.** `சரிபார்க்கப்படாதவை` lists any still marked that way, so
a program can refuse to report a number nobody has vouched for.

An account no rule covers is refused rather than called standard. Standard
provisions least, and that is the one direction a provisioning error must
never go.

## Fabric, without gRPC

Fabric's peer Gateway speaks gRPC, which eTamil does not. `cawkili/fabric.qmz`
talks to a REST gateway instead — Firefly is one, and Hyperledger's own
`fabric-rest-sample` is a reference for writing one. The alternative was months
of HTTP/2 and protobuf to reach the same ledger.

Gateways disagree about their paths, so paths are configuration rather than
something baked in. Two operations, and the difference between them is the
whole of Fabric: `மதிப்பிடு` asks a peer and changes nothing, `சமர்ப்பி`
proposes, endorses, orders and commits.

`மோதலா` is the part worth having. Two transactions that read the same key and
both write it cannot both commit: the second is rejected at validation with
`MVCC_READ_CONFLICT`, *after* endorsement and ordering have already succeeded.
It means "someone got there first", not "this was wrong", and it is the one
Fabric error worth retrying. `மீண்டும்_சமர்ப்பி` retries that and nothing
else — retrying a chaincode refusal turns one rejection into several.

`gateway.py` beside the module is a mock that answers the way a gateway does,
including failing the first write to a contended key. The suite runs against it
when `ETAMIL_FABRIC` names one and skips cleanly when it does not.

## UPI: the public part, and the part that is not

A VPA and the `upi://` link are specified and checkable without anyone's
permission — a QR sticker in a shop is that link and nothing else.
`upi/vilAcam.qmz` builds and checks them, including the two things that catch
people: an amount with more than two decimal places is not money UPI takes,
and a decimal normalises `249.50` to `249.5` when some PSPs hold you to both
places.

**Moving money is not public.** That needs a payment service provider or bank
to sponsor you and NPCI to certify you, and no library substitutes for either.
What is here is everything up to the moment a request leaves for your PSP —
and the mutual TLS and ECDSA signing it will ask for already exist.

`upi/nilYmY.qmz` exists for one rule:

> **PENDING IS NOT FAILURE.**

A request that has not answered may still succeed — the payer's bank may be
slow, the switch may be retrying, the answer may arrive in an hour. Treating
that as failure is how a merchant refunds a payment that then lands, or charges
a customer twice. It is the most common way a UPI integration loses money,
because the wrong behaviour is the one that feels safe.

So two questions are asked separately. `பணம்_வந்ததா` — may I ship? Only a
settled success says yes. `சரிபார்க்கவா` — must I ask again? Anything unsettled
says yes, *including a state nobody recognises*. And `நகர்வு_சரியா` refuses to
let a late or duplicated callback rewrite a settled payment: a success does not
become a failure because a later poll was confused.

## ULI is not here

The Unified Lending Interface is access-controlled, and its interface is not
published in a form anyone outside can build against. Writing a client for it
from guesswork would produce something that looks finished and works against
nothing. When you have the specification through an authorised channel, the
pieces it needs — mutual TLS, ECDSA signing, JSON, a state machine — are all
in place.

## Depreciation, payroll, and where the rates live

Three things worth knowing before using these.

**A schedule closes exactly.** A depreciation charge rounded to the paisa and
taken five times need not equal what is to be written off; the last year takes
the remainder. An asset left with eight paise on the books never closes, the
same way a loan does not.

**A ceiling and an eligibility limit are not the same thing.** Provident fund
contributes on wages *up to* a ceiling. Employees' state insurance contributes
nothing at all above its limit — not a share of the limit. Treating one like
the other takes a deduction from someone who owes none, and the test says so.

**Slabs are marginal.** `படிநிலை_வரி` applies each band's rate only to the part
of the amount inside it. Applying the top rate to the whole amount is what "I
moved up a bracket and took home less" describes, and it is simply wrong. One
engine serves income tax, professional tax and anything else stated in bands —
including bands that carry a flat amount rather than a rate, which is how
professional tax usually reads.

### The rate tables

`vari.qmz` computes a tax once you know the rate. `vari_vikiqam.qmz` is where
the rate comes from, and holds none: GST by HSN, TDS by section, income tax and
professional tax as slabs, VAT where it survives — one effective-dated table,
described in `vari_vikiqam.sql`.

**Every function takes the date it is being asked about, and none defaults it
to today.** A return for last quarter is computed on the rates in force last
quarter; a rate looked up "as of now" quietly rewrites what was filed. A state
rule beats an all-India one, because that is what a state rule is for.

**No rate is seeded. Not one.** What ships is the shape, and the 36 states and
union territories with their GST codes — marked for checking, because a wrong
state code files a return in the wrong state. A missing rate answers a `தவறு`
rather than zero: a rate of nothing and no rate at all are different, and
returning zero for the second understates a liability without saying so.

## Redis is a command, not a query

The roadmap said Redis needed a design before an implementation, because it
does not fit a trait shaped as `execute(sql)` / `query(sql)`. It does not, and
forcing it there would have been the wrong answer: Redis is a command and a
reply, not a query language.

So the host gives exactly one thing — `ரெடிஸ்_கட்டளை(command, arguments)` —
and every Redis command works through it, including ones invented after it was
written. `qaLam/retis.qmz` wraps the handful anybody types.

RESP is implemented in the compiler rather than taken from a crate, for the
same reason the HTTP router was: the protocol is small and a dependency
carrying an async runtime to send `*2
$3
GET
$1
k
`
down a socket is a poor trade.

Two things it gets right that are easy to get wrong. **Arguments are
length-prefixed**, so a value containing CRLF is bytes rather than a second
command — joining arguments with spaces is how command injection works, and it
is not reachable through this. And **a missing key is nil, not `""`**: for a
cache, "absent" and "present and empty" are different questions, and
`வகை(x) == "nil"` tells them apart.

A connection is not shared between requests. Redis keeps state on one — MULTI,
WATCH, SUBSCRIBE — so two requests sharing a connection would interleave a
transaction the way two sharing a SQL connection do. The fix is an exclusive
lease, which the SQL side has and this does not yet.

`retis_pOli.py` is a small mock Redis, so the suite runs on a machine with none
installed.

## Insurance: the average clause

`kAppItu/` is policy, premium and claim. Most of it is small arithmetic. One
part is not, and it is the reason the module exists.

When a property is insured for less than it is worth, the insurer pays only the
proportion that was insured — **even for a partial loss well inside the sum
insured**. A building worth fifty lakh insured for twenty-five is insured for
half its value, so a five-lakh loss pays two and a half lakh, not five.
Policyholders find this surprising, and an implementation that pays the full
partial loss overpays every under-insured claim.

The order of a settlement is also not interchangeable: average on the whole
loss, *then* the excess, then co-payment, then any sub-limit, and the sum
insured as a ceiling throughout. Taking the excess off before averaging pays
more than the policy promises, and a test asserts the two differ.

`கோரல்_தீர்வு` answers a record rather than a number, so a settlement letter can
show how it got there — which is the first thing a policyholder disputing it
will ask.

## Customs: the cascade

`cuwkam/` centres on the order duties are applied in, because that is what
implementations get wrong.

Basic duty is on the assessable value. The surcharge is on **the duty**, not on
the value. And IGST is on the value **plus** the duty and the surcharge. On a
one-lakh consignment at ten per cent duty, computing IGST on the assessable
value instead understates the tax by ₹2,247.30 — and by more the higher the
duty rate. The suite asserts that figure, so the mistake cannot be
reintroduced quietly.

The surcharge is the step most often written against the wrong base, and on the
worked example ten per cent of the value happens to equal the basic duty
exactly — which is why the error is easy to make and hard to spot.

Assessable value is cost, insurance and freight: duty is on the goods delivered
to the border, not on the invoice. An importer computing duty on the invoice
alone underpays on every consignment that had to be shipped.

E-way bill validity **rounds up**. A single kilometre past a distance band earns
the extra day, because rounding down expires a bill while the lorry is still
moving. And `வழிச்சீட்டு_சரிபார்` answers the list of what is missing rather
than a yes or no: the useful answer to "is this valid" is "no, and here is what
to fix", especially with a lorry waiting.

`ஜிஎஸ்டி_எண்_சரியா` checks the **shape** of a GSTIN and says so. The check digit
is a published algorithm and worth adding; claiming to validate a GSTIN without
it would be the more dangerous half-measure.
