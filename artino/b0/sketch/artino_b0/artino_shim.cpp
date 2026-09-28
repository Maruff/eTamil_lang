// SPDX-License-Identifier: MIT
// artino_shim.cpp — the C boundary between LLVM-compiled eTamil and the Arduino core.
//
// Every function here is extern "C" and uses fixed-width types only: `int` is
// 16 bits on AVR and 32 on ARM, and a boundary whose meaning changed with the
// chip would be a boundary that lies. In B1 this file is generated from
// nUlakam/vaZporuL/vaZporuL.qmz; in B0 it is written by hand.

#include <Arduino.h>
#include <stdint.h>

extern "C" {

void artino_serial_begin(int32_t baud) { Serial.begin(baud); }

int32_t artino_led_pin(void) { return LED_BUILTIN; }

void artino_pin_output(int32_t pin) { pinMode(pin, OUTPUT); }

void artino_pin_write(int32_t pin, int32_t on) { digitalWrite(pin, on ? HIGH : LOW); }

uint32_t artino_millis(void) { return millis(); }

void artino_print_text(const char *text) { Serial.print(text); }

void artino_print_line(void) { Serial.println(); }

// A number is its value × 1000. Printed the way the VM prints a decimal:
// no trailing zeros, and no decimal point for a whole number.
// Print has no 64-bit overload on AVR, so the digits are made here.
void artino_print_num(int64_t value) {
  char out[24];
  uint8_t n = 0;
  uint64_t magnitude = value < 0 ? (uint64_t)0 - (uint64_t)value : (uint64_t)value;
  uint64_t whole = magnitude / 1000;
  uint16_t frac = (uint16_t)(magnitude % 1000);

  char digits[20];
  uint8_t d = 0;
  do {
    digits[d++] = (char)('0' + (uint8_t)(whole % 10));
    whole /= 10;
  } while (whole);
  if (value < 0) out[n++] = '-';
  while (d) out[n++] = digits[--d];

  if (frac) {
    out[n++] = '.';
    out[n++] = (char)('0' + frac / 100);
    out[n++] = (char)('0' + (frac / 10) % 10);
    out[n++] = (char)('0' + frac % 10);
    while (out[n - 1] == '0') n--;
  }
  out[n] = '\0';
  Serial.print(out);
}

}  // extern "C"
