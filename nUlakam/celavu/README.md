# celavu — செலவு

Cost accounting: where cost comes from, and where it lands.

The cost sheet builds up from prime cost to cost of sales. Overhead absorption charges estimated overhead onto actual activity and reports what was over- or under-absorbed. Activity-based costing does the same job by driver rather than by a single base, and `குறுக்கு_மானியம்` shows what the single-base method was quietly subsidising.

Standard costing decomposes the difference between plan and outcome into rate, usage, efficiency and volume variances. Process costing handles normal and abnormal loss, and equivalent units under both weighted-average and first-in-first-out.

## Files

| File | What it holds | Functions |
|---|---|---|
| `Uqiya_vERupAtu.qmz` | ஊதிய வேறுபாடு: labour rate, efficiency and idle time | `ஊதிய_மொத்த_வேறுபாடு` `ஊதிய_வீத_வேறுபாடு` `ஊதிய_திறன்_வேறுபாடு` `ஊதிய_செயலிழப்பு_வேறுபாடு` `ஊதிய_வேறுபாட்டுத்_தொகுதி` |
| `celavu_cOqaZY.qmz` | tests for the cost accounting modules | — |
| `celavu_qAL.qmz` | செலவுத் தாள் (the cost sheet) | `முதன்மைச்_செலவு` `நுகர்ந்த_பொருட்கள்` `தொழிற்சாலைச்_செலவு` `உற்பத்திச்_செலவு` `விற்ற_பொருட்களின்_செலவு` `விற்பனைச்_செலவு` `லாபம்_ஈட்டியது` `அலகுக்குச்_செலவு` `செலவுத்_தாள்_ஆக்கு` `தாளை_அச்சிடு` `இருப்பு_ஓட்டம்` |
| `ceyalmuRY.qmz` | செயல்முறை, process costing and equivalent units | `இயல்பு_இழப்பு_அலகுகள்` `அசாதாரண_வேறுபாடு` `நல்ல_வெளியீடு` `உறிஞ்ச_வேண்டிய_செலவு` `அலகுக்கான_செலவு` `அலகுக்கான_செலவு_தவறாக` `அசாதாரண_இழப்பின்_மதிப்பு` `அசாதாரண_ஆதாயத்தின்_மதிப்பு` `செயல்முறையைக்_கணக்கிடு` `சராசரி_சமமான_அலகுகள்` `fifo_சமமான_அலகுகள்` `சமமான_அலகின்_செலவு` `முறைகளின்_வேறுபாடு` |
| `ceyalpAtu.qmz` | செயல்பாடு, activity-based costing | `செயல்பாடு_ஆக்கு` `இயக்கி_விகிதம்` `நுகர்வு_ஆக்கு` `தயாரிப்பின்_மேல்நிலை` `பாரம்பரிய_மேல்நிலை` `தொகுப்புகளின்_கூட்டல்` `குறுக்கு_மானியம்` `பயனளிக்குமா` `மொத்தம்_மாறவில்லையா` |
| `mElnilY.qmz` | மேல்நிலைச் செலவு (overhead absorption) | `அடிப்படை_அலகுகள்` `அடிப்படை_இயந்திர_நேரம்` `அடிப்படை_ஊதிய_நேரம்` `அடிப்படை_நேரடி_ஊதியம்` `அடிப்படை_நேரடிப்_பொருள்` `அடிப்படை_செல்லுபடியா` `உள்வாங்கல்_விகிதம்` `சதவீத_விகிதம்` `உள்வாங்கியது` `உள்வாங்கல்_வேறுபாடு` `அதிக_உள்வாங்கலா` `குறை_உள்வாங்கலா` `நிலையங்களுக்குப்_பகிர்` `சேவையை_மறுபகிர்` |
| `mElnilY_vERupAtu.qmz` | மேல்நிலை வேறுபாடு: overhead, variable and fixed | `மாறும்_மேல்நிலை_மொத்த_வேறுபாடு` `மாறும்_மேல்நிலை_செலவு_வேறுபாடு` `மாறும்_மேல்நிலை_திறன்_வேறுபாடு` `நிலையான_மேல்நிலை_செலவு_வேறுபாடு` `நிலையான_மேல்நிலை_கொள்திறன்_வேறுபாடு` |
| `mILpakirvu.qmz` | மீள்பகிர்வு, reciprocal service apportionment | `சேவை_நிலையம்_ஆக்கு` `பகிர்வு_ஆக்கு` `பகிர்வு_மொத்தம்` `உற்பத்தியா` `நிலையத்_தொகையைப்_பெறு` `நிலையத்_தொகையைக்_கூட்டு` `இருப்புகளின்_கூட்டல்` `இரு_சேவை_ஒரேசமயம்` `மீண்டும்_பகிர்` `படிநிலைப்_பகிர்வு` `உற்பத்திக்கு_மொத்தம்` `நேரடிச்_செலவின்_மொத்தம்` `படிநிலை_வரிசை_வேறுபாடு` |
| `nilYyam.qmz` | நிலையம் (cost centres, objects and elements) | `வகை_உற்பத்தி` `வகை_சேவைத்துறை` `வகை_நிர்வாகம்` `நிலைய_வகை_செல்லுபடியா` `உற்பத்தி_நிலையமா` `நிலையம்_ஆக்கு` `நிலையம்_தேடு` `வகையால்_நிலையங்கள்` `செலவுப்_பொருள்_ஆக்கு` `கூறு_ஆக்கு` `கூறுகளின்_கூட்டல்` `நேரடிக்_கூறுகள்` `மறைமுகக்_கூறுகள்` `மாறும்_பகுதி` `நிலையான_பகுதி` |
| `niyamam.qmz` | நியமச் செலவு (standard costing and variances) | — |
| `pawkaLippu.qmz` | பங்களிப்பு (marginal costing and CVP) | `அலகு_பங்களிப்பு` `மொத்த_பங்களிப்பு` `லாபம்` `பங்களிப்பு_விகிதம்` `சமநிலை_அலகுகள்` `சமநிலை_மதிப்பு` `பாதுகாப்பு_வரம்பு` `பாதுகாப்பு_விகிதம்` `இலக்கு_அலகுகள்` `அலகு_வளத்திற்கு_பங்களிப்பு` |
| `poruL_vERupAtu.qmz` | பொருள் வேறுபாடு: material price and usage | `பொருள்_மொத்த_வேறுபாடு` `பொருள்_வீத_வேறுபாடு` `பொருள்_பயன்பாட்டு_வேறுபாடு` `பொருள்_வேறுபாட்டுத்_தொகுதி` |
| `vERupAtu.qmz` | வேறுபாடு: reading a variance, whichever one it is | `சாதகமா` `பாதகமா` `வேறுபாட்டு_உரை` |
| `viRpaZY_vERupAtu.qmz` | விற்பனை வேறுபாடு: selling price and sales volume | `விற்பனை_வீத_வேறுபாடு` `விற்பனை_அளவு_வேறுபாடு` |
