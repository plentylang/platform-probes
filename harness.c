#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#define INTS int64_t a, int64_t b, int64_t c, int64_t d, int64_t e, int64_t f, int64_t g, int64_t h, int64_t i, int64_t j
#define INT_VALUES 1, -2, 3, -4, 5, -6, 7, -8, 9, -10
#define MIXED int64_t a, double b, int64_t c, double d, int64_t e, double f, int64_t g, double h, int64_t i, double j, int64_t k, double l
#define MIXED_VALUES 1, -2.5, 3, -4.5, 5, -6.5, 7, -8.5, 9, -10.5, 11, -12.5
extern int64_t probe_sum10(INTS);
extern int64_t probe_callback10(INTS);
extern int64_t probe_tail_adapter10(INTS);
extern double probe_mixed(MIXED);
extern double probe_mixed_callback(MIXED);
extern int64_t probe_narrow(int8_t a, uint16_t b);
extern int64_t probe_narrow_callback(int8_t a, uint16_t b);

static void check(int ok, const char *name) {
    if (!ok) { fprintf(stderr, "FAIL: %s\n", name); exit(1); }
}

int64_t host_sum10(INTS) {
    check(a == 1 && b == -2 && c == 3 && d == -4 && e == 5 && f == -6 && g == 7 && h == -8 && i == 9 && j == -10, "C callback integer arguments");
    return a + 2*b + 3*c + 4*d + 5*e + 6*f + 7*g + 8*h + 9*i + 10*j;
}

double host_mixed(MIXED) {
    check(a == 1 && b == -2.5 && c == 3 && d == -4.5 && e == 5 && f == -6.5 && g == 7 && h == -8.5 && i == 9 && j == -10.5 && k == 11 && l == -12.5, "C callback mixed arguments");
    return (double)a + 2*b + 3*(double)c + 4*d + 5*(double)e + 6*f + 7*(double)g + 8*h + 9*(double)i + 10*j + 11*(double)k + 12*l;
}

int64_t host_narrow(int8_t a, uint16_t b) {
    check(a == -101 && b == 60000, "C callback narrow arguments");
    return (int64_t)a + b;
}

int main(void) {
    check(probe_sum10(INT_VALUES) == -55, "C to native integer register/stack arguments");
    check(probe_callback10(INT_VALUES) == -55, "generated to C integer callback return");
    check(probe_tail_adapter10(INT_VALUES) == -55, "native to internal Tail adapter");
    check(probe_mixed(MIXED_VALUES) == -99.0, "C to native mixed register/stack arguments");
    check(probe_mixed_callback(MIXED_VALUES) == -99.0, "generated to C mixed callback return");
    check(probe_narrow(-101, 60000) == 59899, "narrow signed/unsigned arguments");
    check(probe_narrow_callback(-101, 60000) == 59899, "narrow signed/unsigned callback");
    puts("PASS: native AOT ABI probes (integer, mixed float, narrow, C callbacks, Tail adapter)");
    return 0;
}
