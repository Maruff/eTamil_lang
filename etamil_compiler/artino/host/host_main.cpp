// SPDX-License-Identifier: MIT
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//
// The conformance suite's "board": run the program's setup once and stop, so
// its output can be compared line for line with `etamil --vm` on the same
// file. Loop blocks are not run — the VM has no loop to compare them with.
//
// With ARTINO_HOST_WORLD naming a world file, the loop runs too, against a
// script of the outside world, and the output is compared with a .expected
// file. One instruction a line; # starts a comment:
//
//   loops N STEP          run the loop N times, STEP ms apart (default 1 and 0)
//   analog PIN VALUE      what analogRead(PIN) gives from now on
//   pin PIN LEVEL         what digitalRead(PIN) gives from now on
//   line TEXT             TEXT arrives on port 0 as one line
//   node N BODY           serial port N (1-3) has a node on it, which answers
//                         every line the program sends with <NN,BODY*XX, the
//                         checksum worked out here
//   quiet N               node N stops answering (node N again brings it back)
//   at MS <instruction>   do any of these when millis() reaches MS
#include <stdlib.h>

#include <string>

#include "Arduino.h"
#include "artino.h"

HostSerial Serial;

static int levels[256];
static int analogs[256];
static unsigned long now;

void pinMode(int, int) {}
void digitalWrite(int pin, int level) { levels[pin & 255] = level; }
int digitalRead(int pin) { return levels[pin & 255]; }
int analogRead(int pin) { return analogs[pin & 255]; }
void tone(int, unsigned int) {}
void noTone(int) {}
unsigned long millis(void) { return now; }

// --- the nodes on ports 1-3 ------------------------------------------------------

// A serial port with a node on it: what the program writes, whole lines, is
// answered at once with the node's reply.
struct HostPort : public Stream {
  std::string incoming;
  size_t next = 0;
  std::string written;
  std::string reply;  // the body; empty while quiet
  int number = 0;

  int available() override { return (int)(incoming.size() - next); }
  int read() override { return next < incoming.size() ? (unsigned char)incoming[next++] : -1; }
  size_t write(uint8_t byte) override {
    if (byte != '\n') {
      written += (char)byte;
      return 1;
    }
    if (!reply.empty()) {
      char frame[200];
      snprintf(frame, sizeof frame, "%02d,%s", number, reply.c_str());
      unsigned x = 0;
      for (const char *c = frame; *c; c++) x ^= (unsigned char)*c;
      char line[220];
      snprintf(line, sizeof line, "<%s*%02X\n", frame, x);
      incoming.erase(0, next);
      next = 0;
      incoming += line;
    }
    written.clear();
    return 1;
  }
};

static HostPort ports[4];
static bool declared[4];

// A world's nodes are the ports the program can open; there are none without one.
extern "C" Stream *artino_extra_port(int32_t port) {
  return port >= 1 && port <= 3 && declared[port] ? &ports[port] : nullptr;
}
extern "C" void artino_extra_begin(int32_t, int32_t) {}

struct Event {
  unsigned long at;
  char text[200];
};
static Event events[256];
static int event_count;
static char lines[256][160];

// One instruction, now: false if it is not one this file knows.
static bool apply(int index, const char *text) {
  char what[16];
  int pin, value;
  if (sscanf(text, "analog %d %d", &pin, &value) == 2) {
    analogs[pin & 255] = value;
  } else if (sscanf(text, "pin %d %d", &pin, &value) == 2) {
    levels[pin & 255] = value;
  } else if (sscanf(text, "node %d %n", &pin, &value) == 1 && pin >= 1 && pin <= 3) {
    declared[pin] = true;
    ports[pin].number = pin;
    ports[pin].reply = text + value;
  } else if (sscanf(text, "quiet %d", &pin) == 1 && pin >= 1 && pin <= 3) {
    ports[pin].reply.clear();
  } else if (sscanf(text, "%15s", what) == 1 && strcmp(what, "line") == 0) {
    snprintf(lines[index], sizeof lines[index], "%s\n", text + 5);
    Serial.feed(lines[index]);
  } else {
    return false;
  }
  return true;
}

// Every event whose time has come, in the file's order.
static void happen(bool *done) {
  for (int i = 0; i < event_count; i++) {
    if (!done[i] && events[i].at <= now) {
      done[i] = true;
      if (!apply(i, events[i].text)) fprintf(stderr, "world: what is '%s'?\n", events[i].text);
      // One line a loop, so the program reads each before the next arrives.
      if (strncmp(events[i].text, "line", 4) == 0) return;
    }
  }
}

int main() {
  const char *path = getenv("ARTINO_HOST_WORLD");
  if (!path) {
    artino_setup();
    fflush(stdout);
    return 0;
  }
  FILE *world = fopen(path, "r");
  if (!world) {
    fprintf(stderr, "cannot open %s\n", path);
    return 2;
  }
  Serial.world = true;
  long loops = 1, step = 0;
  char text[200];
  while (fgets(text, sizeof text, world)) {
    text[strcspn(text, "\r\n")] = 0;
    if (!text[0] || text[0] == '#') continue;
    unsigned long at = 0;
    int used = 0;
    if (sscanf(text, "loops %ld %ld", &loops, &step) >= 1) continue;
    if (sscanf(text, "at %lu %n", &at, &used) == 1) {
      memmove(text, text + used, strlen(text + used) + 1);
    }
    if (event_count == 256) break;
    events[event_count].at = at;
    snprintf(events[event_count].text, sizeof events[event_count].text, "%s", text);
    event_count++;
  }
  fclose(world);

  static bool done[256];
  happen(done);
  artino_setup();
  for (long i = 0; i < loops; i++) {
    happen(done);
    artino_loop();
    now += step;
  }
  fflush(stdout);
  return 0;
}
