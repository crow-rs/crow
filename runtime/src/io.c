#include "../include/crow_rt.h"
#include <stdio.h>
#include <string.h>

void crow_print_str(CrowStr s) {
    fwrite(s.buff, 1, s.len, stdout);
}

void crow_print_i8(int8_t n)    { printf("%d", (int)n); }
void crow_print_i16(int16_t n)  { printf("%d", (int)n); }
void crow_print_i32(int32_t n)  { printf("%d", n); }
void crow_print_i64(int64_t n)  { printf("%ld", n); }
void crow_print_f32(float n)    { printf("%f", n); }
void crow_print_f64(double n)   { printf("%f", n); }
void crow_print_bool(int8_t b)  { printf("%s", b ? "true" : "false"); }
void crow_println(void)         { printf("\n"); }

CrowStr crow_read_line(void) {
    size_t cap = 128;
    char* buf = crow_alloc(cap);
    size_t len = 0;

    int c;
    while ((c = fgetc(stdin)) != EOF && c != '\n') {
        if (len + 1 >= cap) {
            cap *= 2;
            char* new_buf = crow_alloc(cap);
            memcpy(new_buf, buf, len);
            crow_dealloc(buf);
            buf = new_buf;
        }
        buf[len++] = (char)c;
    }
    buf[len] = '\0';

    CrowStr result;
    result.buff = buf;
    result.len = len;
    return result;
}

int32_t crow_read_i32(void) {
    int32_t n = 0;
    scanf("%d", &n);
    return n;
}

int64_t crow_read_i64(void) {
    int64_t n = 0;
    scanf("%ld", &n);
    return n;
}

double crow_read_f64(void) {
    double n = 0.0;
    scanf("%lf", &n);
    return n;
}