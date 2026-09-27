// artino_b0.ino — the only sketch code a compiled eTamil program needs.
// Everything else is in libartino_prog (built by LLVM) and artino_shim.cpp.
#include <artino_prog.h>

void setup() { artino_setup(); }
void loop() { artino_loop(); }
