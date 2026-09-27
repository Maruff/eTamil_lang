// SPDX-License-Identifier: MIT
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//
// A stand-in for the Arduino API on the machine running the conformance suite
// (scripts/artino_conformance.sh). Serial is stdout, pins remember what was
// written, analog inputs read 0, and time stands still — unless the test gives
// a world file (host_main.cpp), which runs the loop with time, inputs and
// arriving lines. Only what artino_rt.cpp uses exists; it is not an emulator.
#pragma once
#include <stdint.h>
#include <stdio.h>
#include <string.h>

class __FlashStringHelper;
#define F(text) (reinterpret_cast<const __FlashStringHelper *>(text))

#define HIGH 1
#define LOW 0
#define INPUT 0
#define OUTPUT 1
#define INPUT_PULLUP 2

// Just the Stream the runtime reads and writes a serial port through.
class Stream {
 public:
  virtual int available() = 0;
  virtual int read() = 0;
  virtual size_t write(uint8_t byte) = 0;
  size_t write(const uint8_t *bytes, size_t n) {
    size_t written = 0;
    while (n--) written += write(*bytes++);
    return written;
  }
};

// Port 0: stdout, and stdin for what the board would receive, a line at a time.
struct HostSerial : public Stream {
  void begin(long) {}
  void print(const char *text) { fputs(text, stdout); }
  void print(const __FlashStringHelper *text) { fputs(reinterpret_cast<const char *>(text), stdout); }
  void println() { fputc('\n', stdout); }
  void println(const char *text) { print(text); println(); }
  void println(const __FlashStringHelper *text) { print(text); println(); }
  int available() override {
    if (next < length) return (int)(length - next);
    if (fed) {
      strncpy(line, fed, sizeof line - 1);
      line[sizeof line - 1] = 0;
      fed = nullptr;
    } else if (world || !fgets(line, sizeof line, stdin)) {
      return 0;
    }
    length = strlen(line);
    next = 0;
    return (int)length;
  }
  int read() override { return available() > 0 ? (unsigned char)line[next++] : -1; }
  size_t write(uint8_t byte) override {
    fputc(byte, stdout);
    return 1;
  }
  // Under a world file, lines arrive when the file says, not from stdin.
  bool world = false;
  void feed(const char *text) { fed = text; }

 private:
  char line[512];
  size_t length = 0, next = 0;
  const char *fed = nullptr;
};
extern HostSerial Serial;

void pinMode(int pin, int mode);
void digitalWrite(int pin, int level);
int digitalRead(int pin);
int analogRead(int pin);
void tone(int pin, unsigned int hz);
void noTone(int pin);
unsigned long millis(void);
