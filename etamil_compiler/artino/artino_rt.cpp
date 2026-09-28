// SPDX-License-Identifier: MIT
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//
// artino_rt.cpp — the board side of a compiled eTamil program.
//
// Compiled by arduino-cli with the sketch; the program itself arrives as a
// precompiled library built by LLVM. Nothing here allocates: firmware runs for
// months, and a heap that fragments is a board that stops.

#include <Arduino.h>
#include <string.h>
#include "artino.h"
#ifdef __AVR__
#include <avr/wdt.h>
#endif

// Which serial ports the program opens, written by the compiler beside the sketch.
// Without it every port the board has is built in. A port left out is never
// referenced, so its HardwareSerial and 128 bytes of buffers stay off the RAM.
#if defined(__has_include)
#if __has_include("artino_config.h")
#include "artino_config.h"
#endif
#endif
#ifndef ARTINO_TONE
#define ARTINO_TONE 1
#endif
#ifndef ARTINO_LINE_REPORTS
#define ARTINO_LINE_REPORTS 0
#endif
#ifndef ARTINO_PORT1
#define ARTINO_PORT1 1
#endif
#ifndef ARTINO_PORT2
#define ARTINO_PORT2 1
#endif
#ifndef ARTINO_PORT3
#define ARTINO_PORT3 1
#endif

extern "C" {

// --- reports -------------------------------------------------------------------
//
// A number the VM would have computed differently is never silent: the first
// time each site does it, the serial port says what and where. Once, because a
// sample loop that rounds ten times a second would otherwise bury everything
// else the board prints; counted every time, so a program can ask.

enum Kind { ROUNDED_DIV, ROUNDED_MUL, OVERFLOW, DIVIDE_BY_ZERO, OUTSIDE, CUT, UNWRAP, UNWRAP_ERR, SCRIPT, FRACTION };

static uint32_t reports = 0;

uint32_t artino_report_count(void) { return reports; }

// Program text lives in flash on AVR; on ARM flash is ordinary memory.
static void print_program_text(const char *text) { Serial.print((const __FlashStringHelper *)text); }

static char flash_char(const char *at) {
#ifdef __AVR__
  return (char)pgm_read_byte(at);
#else
  return *at;
#endif
}

static uint16_t flash_word(const uint16_t *at) {
#ifdef __AVR__
  return pgm_read_word(at);
#else
  return *at;
#endif
}

static const char *flash_pointer(const char *const *at) {
#ifdef __AVR__
  return (const char *)pgm_read_word(at);
#else
  return *at;
#endif
}

void artino_copy_flash(void *to, const void *from, uint16_t bytes) {
#ifdef __AVR__
  memcpy_P(to, from, bytes);
#else
  memcpy(to, from, bytes);
#endif
}

// A line or site number. Not artino_print_num: a report can come from deep in
// a call chain, and 64-bit formatting would take that much more of the stack.
static void print_small(uint16_t n) {
  char digits[6];
  uint8_t at = sizeof digits - 1;
  digits[at] = '\0';
  do {
    digits[--at] = (char)('0' + n % 10);
    n /= 10;
  } while (n);
  Serial.print(digits + at);
}

static void report(artino_site *site, Kind kind) {
  reports++;
  uint16_t index = flash_word(&site->index);
  uint8_t bit = (uint8_t)(1 << (index & 7));
  if (artino_said[index >> 3] & bit) return;
  artino_said[index >> 3] |= bit;
  Serial.print(F("artino: "));
  const char *where = flash_pointer(&site->where);
  if (where) {
    print_program_text(where);
  } else {
    // artino_sites.txt beside the sketch has this number's full text. The
    // line is given when the compiler knows it.
    uint16_t line = flash_word(&site->line);
    if (line) {
      Serial.print(F("வரி "));
      print_small(line);
      Serial.print(F(", "));
    }
    Serial.print(F("#"));
    print_small(index);
  }
  switch (kind) {
#if ARTINO_LINE_REPORTS
    // A word each, on a board short of flash; docs/artino.md ("Reports") has
    // what each means and what the VM does instead.
    case ROUNDED_DIV:
    case ROUNDED_MUL:
      Serial.println(F(" rounded"));
      break;
    case OVERFLOW:
      Serial.println(F(" overflow"));
      break;
    case DIVIDE_BY_ZERO:
      Serial.println(F(" divided by 0"));
      break;
    case OUTSIDE:
      Serial.println(F(" outside"));
      break;
    case CUT:
      Serial.println(F(" cut"));
      break;
    case UNWRAP:
      Serial.println(F(" unwrap"));
      break;
    case UNWRAP_ERR:
      Serial.println(F(" unwrap error"));
      break;
    case SCRIPT:
      Serial.println(F(" script"));
      break;
    case FRACTION:
      Serial.println(F(" fraction"));
      break;
#else
    case ROUNDED_DIV:
      Serial.println(F(" — needed more than three decimals; rounded (the VM keeps 28 digits)"));
      break;
    case ROUNDED_MUL:
      Serial.println(F(" — needed more than three decimals; rounded (the VM keeps 28 digits)"));
      break;
    case OVERFLOW:
      Serial.println(F(" — too large for a board number; held at the limit"));
      break;
    case DIVIDE_BY_ZERO:
      Serial.println(F(" — division by zero; gave 0 (the VM stops with an error)"));
      break;
    case OUTSIDE:
      Serial.println(F(" — index outside the array; read as 0 or not written (the VM stops with an error)"));
      break;
    case CUT:
      Serial.println(F(" — text longer than 48 bytes; cut to fit (the VM keeps it all)"));
      break;
    case UNWRAP:
      Serial.println(F(" — மதிப்பு of a தவறு; gave the zero value (the VM stops with an error)"));
      break;
    case UNWRAP_ERR:
      Serial.println(F(" — தவறு_மதிப்பு of a சரி; gave empty text (the VM stops with an error)"));
      break;
    case SCRIPT:
      Serial.println(F(" — letters of this script counted one code point each (the VM follows Unicode's rules)"));
      break;
    case FRACTION:
      Serial.println(F(" — a fraction where C++ wants a whole number; passed its whole part"));
      break;
#endif
  }
}

void artino_report_index(artino_site *site) { report(site, OUTSIDE); }

// --- numbers ------------------------------------------------------------------
//
// A number is its value × 1000. The largest is INT64_MAX; the smallest is its
// negation, so every value has one.

static const int64_t SCALE = 1000;
static const int64_t NUM_MAX = INT64_MAX;
static const int64_t NUM_MIN = -INT64_MAX;

static uint64_t magnitude(int64_t v) { return v < 0 ? (uint64_t)0 - (uint64_t)v : (uint64_t)v; }

static int64_t signed_result(uint64_t q, bool negative, artino_site *site) {
  if (q > (uint64_t)NUM_MAX) {
    report(site, OVERFLOW);
    return negative ? NUM_MIN : NUM_MAX;
  }
  return negative ? -(int64_t)q : (int64_t)q;
}

int64_t artino_num_add(int64_t a, int64_t b, artino_site *site) {
  int64_t sum;
  if (__builtin_add_overflow(a, b, &sum) || sum < NUM_MIN) {
    report(site, OVERFLOW);
    return b > 0 ? NUM_MAX : NUM_MIN;
  }
  return sum;
}

int64_t artino_num_sub(int64_t a, int64_t b, artino_site *site) {
  int64_t difference;
  if (__builtin_sub_overflow(a, b, &difference) || difference < NUM_MIN) {
    report(site, OVERFLOW);
    return b < 0 ? NUM_MAX : NUM_MIN;
  }
  return difference;
}

int64_t artino_num_div(int64_t a, int64_t b, artino_site *site) {
  if (b == 0) {
    report(site, DIVIDE_BY_ZERO);
    return 0;
  }
  bool negative = (a < 0) != (b < 0);
  uint64_t n = magnitude(a), d = magnitude(b);

  // Long division, one decimal digit at a time, so no intermediate exceeds
  // ten times the divisor: nothing wider than 64 bits exists on AVR.
  uint64_t whole = n / d, rem = n % d;
  if (whole > (uint64_t)NUM_MAX / SCALE) {
    report(site, OVERFLOW);
    return negative ? NUM_MIN : NUM_MAX;
  }
  uint64_t q = whole * SCALE;
  uint64_t place = SCALE / 10;
  for (int i = 0; i < 3; i++) {
    rem *= 10;
    q += (rem / d) * place;
    rem %= d;
    place /= 10;
  }
  if (rem != 0) {
    // The fourth decimal decides: half away from zero, as வட்டமிடு rounds.
    if ((rem * 10) / d >= 5) q += 1;
    report(site, ROUNDED_DIV);
  }
  return signed_result(q, negative, site);
}

int64_t artino_num_mul_whole(int64_t a, int32_t k, artino_site *site) {
  int64_t product;
  if (__builtin_mul_overflow(a, (int64_t)k, &product) || product == INT64_MIN) {
    report(site, OVERFLOW);
    return (a < 0) != (k < 0) ? NUM_MIN : NUM_MAX;
  }
  return product;
}

int64_t artino_num_mul(int64_t a, int64_t b, artino_site *site) {
  bool negative = (a < 0) != (b < 0);
  uint64_t x = magnitude(a), y = magnitude(b);
  uint64_t q, rest;
  uint64_t product;
  if (!__builtin_mul_overflow(x, y, &product)) {
    q = product / SCALE;
    rest = product % SCALE;
  } else {
    // x × y overflows, but x × y / 1000 may not: split x at the scale.
    uint64_t high;
    if (__builtin_mul_overflow(x / SCALE, y, &high)) {
      report(site, OVERFLOW);
      return negative ? NUM_MIN : NUM_MAX;
    }
    uint64_t low = (x % SCALE) * y;  // x % 1000 < 1000, and y < 2^63: fits
    if (__builtin_add_overflow(high, low / SCALE, &q)) {
      report(site, OVERFLOW);
      return negative ? NUM_MIN : NUM_MAX;
    }
    rest = low % SCALE;
  }
  if (rest != 0) {
    if (rest * 2 >= (uint64_t)SCALE) q += 1;
    report(site, ROUNDED_MUL);
  }
  return signed_result(q, negative, site);
}

// --- printing -----------------------------------------------------------------
//
// The VM's rules: a whole number has no decimal point, trailing zeros go,
// booleans are true and false. Print has no 64-bit overload on AVR, so the
// digits are made here.

void artino_print_text(const char *text) { print_program_text(text); }

// A number as the VM writes it, into out[24]; returns its length.
static uint8_t format_num(char *out, int64_t value) {
  uint8_t n = 0;
  uint64_t m = magnitude(value);
  uint64_t whole = m / SCALE;
  uint16_t frac = (uint16_t)(m % SCALE);
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
  return n;
}

void artino_print_num(int64_t value) {
  char out[24];
  format_num(out, value);
  Serial.print(out);
}

void artino_print_str(const char *text) { Serial.print(text); }

void artino_print_num_array(const int64_t *items, uint16_t count) {
  Serial.print(F("["));
  for (uint16_t i = 0; i < count; i++) {
    if (i) Serial.print(F(", "));
    artino_print_num(items[i]);
  }
  Serial.print(F("]"));
}

void artino_print_bool_array(const uint8_t *items, uint16_t count) {
  Serial.print(F("["));
  for (uint16_t i = 0; i < count; i++) {
    if (i) Serial.print(F(", "));
    artino_print_bool(items[i] & 1);
  }
  Serial.print(F("]"));
}

// --- text ------------------------------------------------------------------------
//
// A text value is a buffer of ARTINO_TEXT_BYTES: at most 48 bytes of UTF-8 and
// a NUL. Nothing grows. What does not fit is cut at a character boundary — a
// Tamil letter is never split into bytes that are not text — and reported.

static const size_t ROOM = ARTINO_TEXT_BYTES - 1;

void artino_text_clear(char *text) { text[0] = '\0'; }

void artino_text_copy(char *destination, const char *source) {
  if (destination != source) memmove(destination, source, strlen(source) + 1);
}

// Undo a cut that left the start of a character without its continuation bytes.
static void end_on_a_character(char *text, size_t used) {
  size_t start = used;
  while (start > 0 && ((uint8_t)text[start - 1] & 0xC0) == 0x80) start--;
  if (start == 0) {
    text[used] = '\0';
    return;
  }
  uint8_t lead = (uint8_t)text[start - 1];
  size_t wanted = lead < 0x80 ? 1 : lead >= 0xF0 ? 4 : lead >= 0xE0 ? 3 : lead >= 0xC0 ? 2 : 1;
  size_t have = used - (start - 1);
  text[have < wanted ? start - 1 : used] = '\0';
}

static void append(char *text, const char *more, bool flash, artino_site *site) {
  size_t used = strlen(text);
  for (size_t i = 0;; i++) {
    char ch = flash ? flash_char(more + i) : more[i];
    if (!ch) {
      text[used] = '\0';
      return;
    }
    if (used == ROOM) {
      end_on_a_character(text, used);
      report(site, CUT);
      return;
    }
    text[used++] = ch;
  }
}

void artino_text_append(char *text, const char *more, artino_site *site) { append(text, more, false, site); }

void artino_text_append_flash(char *text, const char *flash, artino_site *site) { append(text, flash, true, site); }

void artino_text_append_num(char *text, int64_t value, artino_site *site) {
  char digits[24];
  format_num(digits, value);
  append(text, digits, false, site);
}

void artino_text_append_bool(char *text, int32_t value, artino_site *site) {
  append(text, value ? "true" : "false", false, site);
}

int32_t artino_text_equal(const char *a, const char *b) { return strcmp(a, b) == 0; }

void artino_print_bool(int32_t value) {
  if (value) {
    Serial.print(F("true"));
  } else {
    Serial.print(F("false"));
  }
}

void artino_print_line(void) { Serial.println(); }

// --- nUlakam/vaZporuL/vaZporuL.qmz ----------------------------------------------------------

void artino_pin_output(int32_t pin) { pinMode(pin, OUTPUT); }
void artino_pin_input(int32_t pin) { pinMode(pin, INPUT); }
void artino_pin_input_pullup(int32_t pin) { pinMode(pin, INPUT_PULLUP); }
void artino_pin_write(int32_t pin, int32_t on) { digitalWrite(pin, on ? HIGH : LOW); }
int32_t artino_pin_read(int32_t pin) { return digitalRead(pin) == HIGH ? 1 : 0; }
void artino_pin_toggle(int32_t pin) { digitalWrite(pin, digitalRead(pin) == HIGH ? LOW : HIGH); }
int32_t artino_analog_read(int32_t pin) { return analogRead(pin); }
uint32_t artino_millis(void) { return millis(); }

// --- sound and the watchdog -------------------------------------------------------------

// Built only when the program makes a tone: naming tone() links the core's
// Tone, and its timer interrupt stays in the image whether used or not.
#if ARTINO_TONE
void artino_tone(int32_t pin, int32_t hz) { tone(pin, (unsigned int)hz); }
void artino_no_tone(int32_t pin) { noTone(pin); }
#endif

void artino_watchdog_begin(int32_t ms) {
#if defined(__AVR__)
  uint8_t period = ms >= 2000 ? WDTO_2S
                 : ms >= 1000 ? WDTO_1S
                 : ms >= 500 ? WDTO_500MS
                 : ms >= 250 ? WDTO_250MS
                 : ms >= 120 ? WDTO_120MS
                 : ms >= 60 ? WDTO_60MS
                 : ms >= 30 ? WDTO_30MS
                 : WDTO_15MS;
#ifdef WDTO_8S
  if (ms >= 8000) period = WDTO_8S;
  else if (ms >= 4000) period = WDTO_4S;
#endif
  wdt_enable(period);
#elif defined(ARDUINO_ARCH_RP2040)
  rp2040.wdt_begin((uint32_t)ms);
#else
  (void)ms;
#endif
}

void artino_watchdog_feed(void) {
#if defined(__AVR__)
  wdt_reset();
#elif defined(ARDUINO_ARCH_RP2040)
  rp2040.wdt_reset();
#endif
}

// --- results --------------------------------------------------------------------

void artino_report_unwrap(artino_site *site, const char *error) {
  (void)error;
  report(site, UNWRAP);
}

void artino_report_unwrap_err(artino_site *site) { report(site, UNWRAP_ERR); }

static void set_error(artino_result *result, const char *text) {
  result->ok = 0;
  size_t i = 0;
  for (; text[i] && i < ARTINO_TEXT_BYTES - 1; i++) result->payload[i] = text[i];
  result->payload[i] = '\0';
}

static void set_number(artino_result *result, int64_t value) {
  result->ok = 1;
  memcpy(result->payload, &value, sizeof value);
}

void artino_print_result(const artino_result *result, int32_t kind) {
  if (!result->ok) {
    Serial.print(F("தவறு("));
    Serial.print(result->payload);
    Serial.print(F(")"));
    return;
  }
  Serial.print(F("சரி("));
  switch (kind) {
    case 0: {
      int64_t value;
      memcpy(&value, result->payload, sizeof value);
      artino_print_num(value);
      break;
    }
    case 1:
      artino_print_bool(result->payload[0] & 1);
      break;
    case 2:
      Serial.print(result->payload);
      break;
    default:
      Serial.print(F("nil"));
  }
  Serial.print(F(")"));
}

// --- rounding the author asked for ---------------------------------------------------

// 10^n. A table would be 56 bytes of RAM on AVR.
static uint64_t ten_to(uint8_t n) {
  uint64_t value = 1;
  while (n--) value *= 10;
  return value;
}

// |q| rounded by mode, given what was left over (rest of a divisor `unit`).
static uint64_t settle(uint64_t q, uint64_t rest, uint64_t unit, bool negative, int32_t mode) {
  if (rest == 0) return q;
  switch (mode) {
    case 0:
      return negative ? q + 1 : q;  // floor: away from zero only below it
    case 1:
      return negative ? q : q + 1;  // ceiling
    default:
      return rest * 2 >= unit ? q + 1 : q;  // half away from zero
  }
}

int64_t artino_num_round(int64_t value, int32_t places, int32_t mode) {
  if (places >= 3) return value;
  bool negative = value < 0;
  uint64_t unit = ten_to(3 - places);
  uint64_t m = magnitude(value);
  uint64_t q = settle(m / unit, m % unit, unit, negative, mode) * unit;
  return negative ? -(int64_t)q : (int64_t)q;
}

int64_t artino_num_div_round(int64_t a, int64_t b, int32_t places, int32_t mode, artino_site *site) {
  if (b == 0) {
    report(site, DIVIDE_BY_ZERO);
    return 0;
  }
  if (places > 3) places = 3;
  bool negative = (a < 0) != (b < 0);
  uint64_t n = magnitude(a), d = magnitude(b);
  // n / d is the true value; its digits to `places` decimals, then the rest decides.
  uint64_t q = n / d, rem = n % d;
  for (int32_t i = 0; i < places; i++) {
    rem *= 10;
    q = q * 10 + rem / d;
    rem %= d;
  }
  q = settle(q, rem, d, negative, mode);
  uint64_t scaled;
  if (__builtin_mul_overflow(q, ten_to(3 - places), &scaled) || scaled > (uint64_t)NUM_MAX) {
    report(site, OVERFLOW);
    return negative ? NUM_MIN : NUM_MAX;
  }
  return negative ? -(int64_t)scaled : (int64_t)scaled;
}

int64_t artino_num_mul_round(int64_t a, int64_t b, int32_t places, int32_t mode, artino_site *site) {
  if (places > 3) places = 3;
  bool negative = (a < 0) != (b < 0);
  uint64_t x = magnitude(a), y = magnitude(b), product;
  if (__builtin_mul_overflow(x, y, &product)) {
    // Too wide to settle exactly: multiply as usual, which reports, then round.
    return artino_num_round(artino_num_mul(a, b, site), places, mode);
  }
  // product is the value × 10^6; settle it at 10^places.
  uint64_t unit = ten_to(6 - places);
  uint64_t q = settle(product / unit, product % unit, unit, negative, mode);
  uint64_t scaled;
  if (__builtin_mul_overflow(q, ten_to(3 - places), &scaled) || scaled > (uint64_t)NUM_MAX) {
    report(site, OVERFLOW);
    return negative ? NUM_MIN : NUM_MAX;
  }
  return negative ? -(int64_t)scaled : (int64_t)scaled;
}

// --- எண்ணாக்கு -----------------------------------------------------------------------

static bool space(char ch) { return ch == ' ' || ch == '\t' || ch == '\r' || ch == '\n'; }

void artino_text_to_number(const char *text, artino_result *result, artino_site *site) {
  const char *at = text;
  while (space(*at)) at++;
  const char *end = text + strlen(text);
  while (end > at && space(end[-1])) end--;

  bool negative = false;
  if (at < end && (*at == '+' || *at == '-')) negative = *at++ == '-';
  uint64_t whole = 0, frac = 0;
  int digits = 0, decimals = 0;
  bool point = false, rounded = false, carry = false, too_big = false;
  for (; at < end; at++) {
    char ch = *at;
    if (ch == '.' && !point) {
      point = true;
      continue;
    }
    if (ch < '0' || ch > '9') break;
    digits++;
    if (!point) {
      if (whole > (uint64_t)NUM_MAX / 10000) too_big = true;
      whole = whole * 10 + (uint64_t)(ch - '0');
    } else if (decimals < 3) {
      frac = frac * 10 + (uint64_t)(ch - '0');
      decimals++;
    } else {
      if (decimals == 3) carry = ch >= '5';  // the fourth decimal decides
      if (ch != '0') rounded = true;
      decimals++;
    }
  }
  if (at != end || digits == 0) {
    // The Tamil half of the VM's message: its full two-language text does not
    // fit a board's 48 bytes.
    char message[ARTINO_TEXT_BYTES];
    size_t n = 0;
    message[n++] = '\'';
    for (size_t i = 0; text[i] && n < 12; i++) message[n++] = text[i];
    message[n++] = '\'';
    message[n] = '\0';
    strncat(message, " ஒரு எண் அல்ல", ARTINO_TEXT_BYTES - 1 - n);
    set_error(result, message);
    return;
  }
  if (too_big) {
    report(site, OVERFLOW);
    set_number(result, negative ? NUM_MIN : NUM_MAX);
    return;
  }
  while (decimals < 3) {
    frac *= 10;
    decimals++;
  }
  uint64_t value = whole * SCALE + frac + (carry ? 1 : 0);
  if (rounded || carry) report(site, ROUNDED_DIV);
  set_number(result, negative ? -(int64_t)value : (int64_t)value);
}

// --- letters -------------------------------------------------------------------------

static uint8_t decode(const char *at, uint32_t *cp) {
  uint8_t lead = (uint8_t)at[0];
  if (lead < 0x80) {
    *cp = lead;
    return 1;
  }
  uint8_t length = lead >= 0xF0 ? 4 : lead >= 0xE0 ? 3 : 2;
  uint32_t value = lead & (0x3F >> (length - 1));
  for (uint8_t i = 1; i < length; i++) {
    if (!at[i]) {
      *cp = 0xFFFD;
      return i;
    }
    value = (value << 6) | ((uint8_t)at[i] & 0x3F);
  }
  *cp = value;
  return length;
}

// Joins the letter before it: Tamil vowel signs, virama, anusvara and au
// length mark, combining diacritics, joiners and variation selectors.
static bool extends(uint32_t cp) {
  return cp == 0x0B82 || (cp >= 0x0BBE && cp <= 0x0BCD) || cp == 0x0BD7 || (cp >= 0x0300 && cp <= 0x036F) ||
         cp == 0x200C || cp == 0x200D || (cp >= 0xFE00 && cp <= 0xFE0F);
}

// Scripts whose letters this counts exactly as the VM does.
static bool known(uint32_t cp) {
  return cp < 0x0250 || (cp >= 0x0300 && cp <= 0x036F) || (cp >= 0x0B80 && cp <= 0x0BFF) ||
         (cp >= 0x2000 && cp <= 0x206F) || (cp >= 0x20A0 && cp <= 0x20CF) || (cp >= 0xFE00 && cp <= 0xFE0F);
}

// Walk letters: calls back with each letter's start and byte length. Returns the count.
static int32_t letters(const char *text, int32_t wanted, char *out, artino_site *site) {
  int32_t count = 0;
  size_t i = 0;
  while (text[i]) {
    size_t start = i;
    uint32_t cp;
    i += decode(text + i, &cp);
    if (!known(cp)) report(site, SCRIPT);
    if (cp == '\r' && text[i] == '\n') i++;
    while (text[i]) {
      uint32_t next;
      uint8_t length = decode(text + i, &next);
      if (!extends(next)) break;
      i += length;
    }
    if (count == wanted && out) {
      size_t n = i - start;
      bool cut = n > ARTINO_LETTER_BYTES - 1;
      if (cut) n = ARTINO_LETTER_BYTES - 1;
      memcpy(out, text + start, n);
      out[n] = '\0';
      if (cut) {
        end_on_a_character(out, n);
        report(site, CUT);
      }
    }
    count++;
  }
  return count;
}

int32_t artino_text_letters(const char *text, artino_site *site) { return letters(text, -1, nullptr, site); }

void artino_text_letter(const char *text, int32_t index, char *out, artino_site *site) {
  out[0] = '\0';
  int32_t count = letters(text, index, out, site);
  if (index < 0 || index >= count) report(site, OUTSIDE);
}

void artino_text_letter_flash(const char *flash, int32_t index, char *out, artino_site *site) {
  char text[ARTINO_TEXT_BYTES];
  artino_copy_flash(text, flash, sizeof text);
  artino_text_letter(text, index, out, site);
}

// --- C++, through a manifest ------------------------------------------------------------

int32_t artino_to_int(int64_t value, artino_site *site) {
  int64_t whole = value / SCALE;
  if (whole > INT32_MAX || whole < INT32_MIN) {
    report(site, OVERFLOW);
    return whole > 0 ? INT32_MAX : INT32_MIN;
  }
  if (value % SCALE != 0) report(site, FRACTION);
  return (int32_t)whole;
}

void artino_text_set(char *text, const char *from) {
  size_t n = strlen(from);
  if (n > ARTINO_TEXT_BYTES - 1) n = ARTINO_TEXT_BYTES - 1;
  memcpy(text, from, n);
  text[n] = '\0';
  end_on_a_character(text, n);
}

// A manifest's [[port]]s, defined in the sketch's artino_shims.cpp. Without
// one there are none.
__attribute__((weak)) Stream *artino_extra_port(int32_t) { return nullptr; }
__attribute__((weak)) void artino_extra_begin(int32_t, int32_t) {}

// --- serial ports ----------------------------------------------------------------------

static Stream *port_stream(int32_t port) {
  switch (port) {
    case 0:
      return &Serial;
#if (defined(HAVE_HWSERIAL1) || defined(ARDUINO_ARCH_RP2040)) && ARTINO_PORT1
    case 1:
      return &Serial1;
#endif
#if (defined(HAVE_HWSERIAL2) || defined(ARDUINO_ARCH_RP2040)) && ARTINO_PORT2
    case 2:
      return &Serial2;
#endif
#if defined(HAVE_HWSERIAL3) && ARTINO_PORT3
    case 3:
      return &Serial3;
#endif
    default:
      return artino_extra_port(port);
  }
}

static void port_begin(int32_t port, int32_t baud) {
  switch (port) {
    case 0:
      Serial.begin(baud);
      break;
#if (defined(HAVE_HWSERIAL1) || defined(ARDUINO_ARCH_RP2040)) && ARTINO_PORT1
    case 1:
      Serial1.begin(baud);
      break;
#endif
#if (defined(HAVE_HWSERIAL2) || defined(ARDUINO_ARCH_RP2040)) && ARTINO_PORT2
    case 2:
      Serial2.begin(baud);
      break;
#endif
#if defined(HAVE_HWSERIAL3) && ARTINO_PORT3
    case 3:
      Serial3.begin(baud);
      break;
#endif
    default:
      artino_extra_begin(port, baud);
      break;
  }
}

// Port 0 is open from power-on: the sketch starts Serial before the program.
// Buffers for ports 0 up to the highest one the program opens, and no more:
// each is a text's worth of RAM. A port past them has no stream, so
// port_stream refuses it before any of these is indexed.
#if ARTINO_PORT3
#define PORTS 4
#elif ARTINO_PORT2
#define PORTS 3
#elif ARTINO_PORT1
#define PORTS 2
#else
#define PORTS 1
#endif
static bool opened[PORTS] = {true};
static char pending[PORTS][ARTINO_TEXT_BYTES];
static uint8_t pending_length[PORTS];
static bool pending_cut[PORTS];

static void port_message(char *out, int32_t port, const char *tail) {
  // "தொடர் துறை N …" in a board's 48 bytes.
  const char *head = "துறை ";
  size_t n = 0;
  for (size_t i = 0; head[i]; i++) out[n++] = head[i];
  out[n++] = (char)('0' + (port >= 0 && port <= 9 ? port : 0));
  out[n++] = ' ';
  for (size_t i = 0; tail[i] && n < ARTINO_TEXT_BYTES - 1; i++) out[n++] = tail[i];
  out[n] = '\0';
}

static bool usable(int32_t port, artino_result *result) {
  char message[ARTINO_TEXT_BYTES];
  if (!port_stream(port)) {
    port_message(message, port, "இல்லை");
    set_error(result, message);
    return false;
  }
  if (!opened[port]) {
    port_message(message, port, "மூடியுள்ளது");
    set_error(result, message);
    return false;
  }
  return true;
}

void artino_serial_open(int32_t port, int32_t baud, artino_result *result) {
  if (!port_stream(port)) {
    char message[ARTINO_TEXT_BYTES];
    port_message(message, port, "இல்லை");
    set_error(result, message);
    return;
  }
  port_begin(port, baud);
  opened[port] = true;
  pending_length[port] = 0;
  pending_cut[port] = false;
  set_number(result, (int64_t)port * SCALE);
}

void artino_serial_read_line(int32_t port, int32_t wait, artino_result *result, artino_site *site) {
  if (!usable(port, result)) return;
  Stream *stream = port_stream(port);
  uint32_t start = millis();
  for (;;) {
    while (stream->available() > 0) {
      int ch = stream->read();
      if (ch < 0) break;
      if (ch == '\r') continue;
      if (ch == '\n') {
        pending[port][pending_length[port]] = '\0';
        if (pending_cut[port]) {
          end_on_a_character(pending[port], pending_length[port]);
          report(site, CUT);
        }
        result->ok = 1;
        memcpy(result->payload, pending[port], strlen(pending[port]) + 1);
        pending_length[port] = 0;
        pending_cut[port] = false;
        return;
      }
      if (pending_length[port] < ARTINO_TEXT_BYTES - 1) {
        pending[port][pending_length[port]++] = (char)ch;
      } else {
        pending_cut[port] = true;
      }
    }
    if (wait <= 0 || (uint32_t)(millis() - start) >= (uint32_t)wait) break;
  }
  // No whole line yet: சரி(""), as vaZporuL answers on every board.
  result->ok = 1;
  result->payload[0] = '\0';
}

void artino_serial_write(int32_t port, const char *text, int32_t newline, artino_result *result) {
  if (!usable(port, result)) return;
  Stream *stream = port_stream(port);
  size_t n = strlen(text);
  size_t written = stream->write((const uint8_t *)text, n);
  if (newline) written += stream->write((uint8_t)'\n');
  set_number(result, (int64_t)written * SCALE);
}

void artino_serial_close(int32_t port, artino_result *result) {
  if (!usable(port, result)) return;
  opened[port] = port == 0;  // Serial stays: it is the board's own voice
  result->ok = 1;
  result->payload[0] = '\0';
}

}  // extern "C"
