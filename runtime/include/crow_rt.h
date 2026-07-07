#ifndef CROW_RT_H
#define CROW_RT_H

#include <stdint.h>
#include <stddef.h>

typedef struct {
    const char* buff;
    size_t len;
} CrowStr;

extern int8_t crow_main(void);

_Noreturn void crow_panic(CrowStr msg);
_Noreturn void crow_abort(void);

void crow_print_str(CrowStr s);
void crow_print_i8(int8_t n);
void crow_print_i16(int16_t n);
void crow_print_i32(int32_t n);
void crow_print_i64(int64_t n);
void crow_print_f32(float n);
void crow_print_f64(double n);
void crow_print_bool(int8_t b);
void crow_println(void);

CrowStr crow_read_line(void);
int32_t crow_read_i32(void);
int64_t crow_read_i64(void);
double crow_read_f64(void);

void* crow_alloc(size_t size);
void  crow_dealloc(void* ptr);

CrowStr crow_str_concat(CrowStr a, CrowStr b);
size_t  crow_str_len(CrowStr s);
int8_t  crow_str_eq(CrowStr a, CrowStr b);

#endif