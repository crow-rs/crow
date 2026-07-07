#include "../include/crow_rt.h"
#include <stdlib.h>
#include <stdio.h>

int main(void) {
    printf("Called CRT0\n");
    int8_t result = crow_main();
    return (int)result;
}