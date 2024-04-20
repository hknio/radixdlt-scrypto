// trace-pc-guard-cb.cc
#include <stdint.h>
#include <stdio.h>
#include <sanitizer/coverage_interface.h>

uint8_t *START = 0;
uint8_t *STOP = 0;

extern "C" void __sanitizer_cov_8bit_counters_init(uint8_t *start,
                                                    uint8_t *stop) {
    printf("Called, len: %i\n", stop - start);
    START = start;
    STOP = stop;
}

void foo() { }
extern "C" int work(int argc) {
  if (argc > 1) foo();
  for (uint8_t *x = START; x < STOP; x++) {
    printf("hit: %i\n", *x);
  }
}
