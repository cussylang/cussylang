#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>

int main(void) {
    int64_t x = 12345;
    for (int64_t i = 0; i < 200000; i++) {
        x = (x * 1664525 + 1013904223) % 2147483647;
    }
    printf("%" PRId64 "\n", x);
    return 0;
}
