# vaZporuL — வன்பொருள்

Hardware: pins, time and serial ports, behind one API.

The same program runs against a Raspberry Pi's GPIO, a simulated board, or an Arduino through artino — `பலகை()` says which. The `போலி_*` functions drive the simulated board, so hardware logic can be tested with `etamil --vm` before anything is uploaded.

The board files each name one board's pins under the same Tamil names, so a program written for an Uno reads the same as one written for a Pico.

## Files

| File | What it holds | Functions |
|---|---|---|
| `mekA.qmz` | மெகா: the Arduino Mega 2560's pins, and the hardware API | — |
| `muZY.qmz` | முனை: the pins, and which board this is | `பலகை` `முனை_வெளியீடு` `முனை_உள்ளீடு` `முனை_மேலிழு_உள்ளீடு` `முனை_எழுது` `முனை_படி` `முனை_மாற்று` `ஒப்புமை_படி` |
| `nAnO.qmz` | நானோ: the Arduino Nano's pins, and the hardware API | — |
| `nEram.qmz` | நேரம்: time, sound and the watchdog | `மில்லி_நொடி` `காத்திரு` `ஒலி_எழுப்பு` `ஒலி_நிறுத்து` `காவல்_தொடங்கு` `காவல்_புதுப்பி` |
| `pIkO.qmz` | பீகோ: the Raspberry Pi Pico's pins, and the hardware API | — |
| `pOli.qmz` | போலி: driving the simulated board, for tests | `போலி_முனை` `போலி_ஒப்புமை` `போலி_நேரம்` `போலி_தொடர்_ஊட்டு` `போலி_தொடர்_வெளியீடு` |
| `qotarqquRY.qmz` | தொடர்த்துறை: serial ports | `தொடர்_திற` `தொடர்_வரி_படி` `தொடர்_எழுது` `தொடர்_வரி_எழுது` `தொடர்_மூடு` |
| `rAspY.qmz` | ராஸ்பை: the Raspberry Pi's pins, and the hardware API | `தலைப்பு_முனை` |
| `vaZporuL.qmz` | வன்பொருள் (hardware): pins, time, serial | — |
| `vaZporuL_cOqaZY.qmz` | tests for pins, time and serial on the simulated board | — |
| `yUnO.qmz` | யூனோ: the Arduino Uno's pins, and the hardware API | — |

## uqavi — உதவி (worked examples)

`uqavi/` holds 25 runnable programs, one for each function above, named
after the function they demonstrate. Each shows the ordinary use and the
cases that are easy to get wrong, with the reasoning in English and Tamil.

They are run by `scripts/run_samples.sh`, so a sample that stops matching
its function fails rather than quietly going stale:

```
./scripts/run_samples.sh vaZporuL
```
