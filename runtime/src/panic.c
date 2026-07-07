#include "../include/crow_rt.h"
#include <stdio.h>
#include <stdlib.h>

_Noreturn void crow_panic(CrowStr msg) {
    fprintf(stderr, "panic: TODO MESSAGE\n");
    abort();
}

_Noreturn void crow_abort(void) {
    abort();
}