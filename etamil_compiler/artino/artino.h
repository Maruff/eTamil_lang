// SPDX-License-Identifier: MIT
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//
// artino.h — what a compiled eTamil program calls on the board.
//
// Written beside every sketch `etamil --artino` produces, with artino_rt.cpp.
// MIT, unlike the compiler, so firmware built with artino is its author's to
// license (docs/artino.md, "Licence").
//
// Every function is extern "C" and uses fixed-width types only: `int` is 16
// bits on AVR and 32 on ARM, and a boundary whose meaning changed with the chip
// would be a boundary that lies. A number crosses it as int64_t holding the
// value × 1000.
#pragma once
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// The program, compiled by LLVM into libartino_prog.
void artino_setup(void);
void artino_loop(void);

// One place in the program where arithmetic can come out differently from the
// VM. The compiler makes one per operation, constant and kept in flash on AVR:
// its number, its line, and `where`, its text — or NULL on a board short of
// flash (Uno, Nano), where the sketch's artino_sites.txt has the text of each
// number. Each site is reported once, however often it runs.
typedef struct {
  uint16_t index;
  uint16_t line;
  const char *where;
} artino_site;

// One bit per site, set once it has been reported. The program sizes it.
extern uint8_t artino_said[];

// A number where C++ wants an int32: its whole part. A fraction, or a number
// past int32, is reported.
int32_t artino_to_int(int64_t value, artino_site *site);

// Text from C++ into a program buffer, cut to fit at a character.
void artino_text_set(char *text, const char *from);

// A `நிலை` text or array the program keeps in flash, copied out to be read.
void artino_copy_flash(void *to, const void *from, uint16_t bytes);

// Numbers, on value × 1000. Addition and subtraction saturate on overflow;
// multiplication and division round half away from zero past three decimals.
// Every one of those — and a division by zero, which gives 0 — is reported.
int64_t artino_num_add(int64_t a, int64_t b, artino_site *site);
int64_t artino_num_sub(int64_t a, int64_t b, artino_site *site);
int64_t artino_num_mul(int64_t a, int64_t b, artino_site *site);
// By a whole number: exact, so only overflow is reported.
int64_t artino_num_mul_whole(int64_t a, int32_t k, artino_site *site);
int64_t artino_num_div(int64_t a, int64_t b, artino_site *site);

// How many reports have been made, including the ones not printed again.
uint32_t artino_report_count(void);

// An array index outside its array: reported; a read gives 0, a write is skipped.
void artino_report_index(artino_site *site);

// Text: a buffer of ARTINO_TEXT_BYTES, NUL-terminated UTF-8. The compiler's
// analyse::TEXT_BYTES must agree. Appending past the end cuts at a character
// boundary and reports, once per site.
#define ARTINO_TEXT_BYTES 49
void artino_text_clear(char *text);
void artino_text_copy(char *destination, const char *source);
void artino_text_append(char *text, const char *more, artino_site *site);
void artino_text_append_flash(char *text, const char *flash, artino_site *site);
void artino_text_append_num(char *text, int64_t value, artino_site *site);
void artino_text_append_bool(char *text, int32_t value, artino_site *site);
int32_t artino_text_equal(const char *a, const char *b);

// அச்சு. print_text is a literal from the program, in flash on AVR;
// print_str is text in RAM. Arrays print as the VM prints them: [1, 2, 3].
void artino_print_text(const char *text);
void artino_print_str(const char *text);
void artino_print_num_array(const int64_t *items, uint16_t count);
void artino_print_bool_array(const uint8_t *items, uint16_t count);
void artino_print_num(int64_t value);
void artino_print_bool(int32_t value);
void artino_print_line(void);

// A result: the value when ok, the error text when not, in one payload. The
// compiler's result layout, { i1 ok, [49 x i8] payload }, is this struct.
typedef struct {
  uint8_t ok;
  char payload[ARTINO_TEXT_BYTES];
} artino_result;

// மதிப்பு of a தவறு, and தவறு_மதிப்பு of a சரி: reported, and the zero given.
void artino_report_unwrap(artino_site *site, const char *error);
void artino_report_unwrap_err(artino_site *site);

// அச்சு of a result: சரி(…) or தவறு(…). kind: 0 number, 1 boolean, 2 text, 3 nothing.
void artino_print_result(const artino_result *result, int32_t kind);

// தரை, மேல், வட்டமிடு. mode: 0 floor, 1 ceiling, 2 half away from zero;
// places 0 to 3. Division and multiplication inside them are done exactly,
// straight to the rounded answer, and not reported: the author asked for it.
int64_t artino_num_round(int64_t value, int32_t places, int32_t mode);
int64_t artino_num_div_round(int64_t a, int64_t b, int32_t places, int32_t mode, artino_site *site);
int64_t artino_num_mul_round(int64_t a, int64_t b, int32_t places, int32_t mode, artino_site *site);

// எண்ணாக்கு: a number, or a தவறு.
void artino_text_to_number(const char *text, artino_result *result, artino_site *site);

// Letters, as the VM counts them for Latin and Tamil text: a letter is a base
// character with the vowel signs, virama and combining marks after it, and
// \r\n is one. Other scripts are counted one code point each, and reported.
// One letter: a base character and what joins it, which is never near a
// text's 48 bytes. A longer one is cut and reported.
#define ARTINO_LETTER_BYTES 16

int32_t artino_text_letters(const char *text, artino_site *site);
void artino_text_letter(const char *text, int32_t index, char *out, artino_site *site);
void artino_text_letter_flash(const char *flash, int32_t index, char *out, artino_site *site);

// Serial ports: 0 is Serial, 1-3 the board's hardware ports where it has them.
// Opening gives சரி(port) or தவறு; reading gives the next whole line, or ""
// when none has arrived — waiting up to `wait` ms, 0 not at all.
void artino_serial_open(int32_t port, int32_t baud, artino_result *result);
void artino_serial_read_line(int32_t port, int32_t wait, artino_result *result, artino_site *site);
void artino_serial_write(int32_t port, const char *text, int32_t newline, artino_result *result);
void artino_serial_close(int32_t port, artino_result *result);

// nUlakam/vaZporuL.qmz
void artino_pin_output(int32_t pin);
void artino_pin_input(int32_t pin);
void artino_pin_input_pullup(int32_t pin);
void artino_pin_write(int32_t pin, int32_t on);
int32_t artino_pin_read(int32_t pin);
void artino_pin_toggle(int32_t pin);
int32_t artino_analog_read(int32_t pin);
uint32_t artino_millis(void);

// A square wave on a pin, for a speaker, until artino_no_tone.
void artino_tone(int32_t pin, int32_t hz);
void artino_no_tone(int32_t pin);

// Restart the board unless artino_watchdog_feed comes within ms: the longest
// period the chip has that is not longer, so it never restarts late.
void artino_watchdog_begin(int32_t ms);
void artino_watchdog_feed(void);

#ifdef __cplusplus
}
#endif
