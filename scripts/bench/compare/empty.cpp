// Startup only, so it can be subtracted. Same runtime surface as the tax
// programs beside it — <cstdio> and <string>, no iostream — so that what is
// subtracted is this toolchain's startup and not a different one's.
#include <cstdio>
#include <string>
int main() { std::printf("0\n"); return 0; }
