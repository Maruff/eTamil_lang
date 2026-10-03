// C++ counterpart of tax_double.c — binary float, the fast and inexact half.
#include <cstdio>
#include <cstdint>
#include <string>

int main(int argc, char **argv) {
    if (argc < 2) { std::fprintf(stderr, "need N\n"); return 2; }
    const std::int64_t n = std::stoll(argv[1]);

    double total = 0.0;
    for (std::int64_t i = 0; i < n; ++i) {
        const double income = 300000.0 + static_cast<double>(i);
        const double tax = (income - 300000.0) * 0.05;
        total += tax;
    }
    std::printf("%.2f\n", total);
    return 0;
}
