#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>

static bool prime(int64_t n) {
    if (n < 2) { return false; }
    for (int64_t d = 2; d * d <= n; d++) {
        if (n % d == 0) { return false; }
    }
    return true;
}

int main(void) {
    int64_t count = 0;
    for (int64_t n = 2; n <= 3000; n++) {
        if (prime(n)) { count++; }
    }
    printf("%" PRId64 "\n", count);
    return 0;
}
