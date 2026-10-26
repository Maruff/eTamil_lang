# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
"""A stand-in for `etamil --repl` that follows the same prompt protocol.

It exists so the session driver can be tested for its protocol handling, including
cases the real shell makes awkward to produce on demand (dying mid-cell).

    say TEXT      prints TEXT (a result, on stdout)
    fail          writes `✗ it failed` to stderr (an error)
    quiet         prints nothing
    :quit         exits
    anything ending in an unclosed {   keeps reading, as the real shell does
    a block that closes   prints `block ran`

Set FAKE_CRLF=1 to end every line with CRLF, as some Windows tools do.
"""

import os
import sys

# Exact bytes, whatever the platform's defaults: UTF-8 both ways, and LF line ends
# unless the test asks for CRLF.
EOL = "\r\n" if os.environ.get("FAKE_CRLF") else "\n"
sys.stdin.reconfigure(encoding="utf-8")
sys.stdout.reconfigure(encoding="utf-8", newline="")
sys.stderr.reconfigure(encoding="utf-8", newline="")


def out(text, stream=sys.stdout):
    stream.write(text)
    stream.flush()


def open_braces(source):
    return max(source.count("{") - source.count("}"), 0)


out("fake eTamil. :quit to leave." + EOL)
pending = ""
while True:
    out("…  " if pending else "» ")
    line = sys.stdin.readline()
    if not line:
        out(EOL)
        sys.exit(0)
    line = line.rstrip("\r\n")
    if not pending:
        if line.strip() == "":
            continue
        if line.strip() == ":quit":
            sys.exit(0)
    pending += line + "\n"
    if open_braces(pending) > 0:
        continue
    source, pending = pending.strip(), ""
    if source.startswith("say "):
        out(source[4:] + EOL)
    elif source == "fail":
        out("✗ it failed" + EOL, sys.stderr)
    elif source.endswith("}"):
        out("block ran" + EOL)
