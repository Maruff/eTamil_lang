// C++ counterpart of scripts/bench/compare/tax_int.c — exact money as int64
// scaled to paisa, because C++ has no decimal type either.
//
// Same algorithm and same I/O as the C version on purpose: the question being
// answered is whether C++ differs from C on this workload, and changing the
// loop as well as the language would make that unanswerable.
//
// N comes from argv for the reason the C file gives: at /O2 a literal bound is
// folded at compile time and the benchmark becomes process startup.
#include <cstdio>
#include <cstdint>
#include <string>

int main(int argc, char **argv) {
    if (argc < 2) { std::fprintf(stderr, "need N\n"); return 2; }
    const std::int64_t n = std::stoll(argv[1]);

    std::int64_t total = 0;                                  // paisa
    for (std::int64_t i = 0; i < n; ++i) {
        const std::int64_t income = 30000000 + i * 100;      // paisa
        total += ((income - 30000000) * 5) / 100;
    }

    const std::int64_t whole = total / 100, frac = total % 100;
    if (frac == 0) std::printf("%lld\n", static_cast<long long>(whole));
    else           std::printf("%lld.%02lld\n", static_cast<long long>(whole),
                               static_cast<long long>(frac));
    return 0;
}
