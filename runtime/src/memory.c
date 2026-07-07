#include "../include/crow_rt.h"
#include <stdlib.h>
#include <stdio.h>

// MVP, todo GC ref counted
void* crow_alloc(size_t size) {
    void* ptr = malloc(size);
    if (!ptr) {
        exit(-1);
        //crow_panic(CrowStr {}); // todo
    }
    return ptr;
}

void crow_dealloc(void* ptr) {
    free(ptr);
}

