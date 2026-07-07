#include "../include/crow_rt.h"
#include <string.h>

CrowStr crow_str_concat(CrowStr a, CrowStr b) {
    size_t la = a.len;
    size_t lb = b.len;
    size_t result_len = la + lb + 1;
    char* result = crow_alloc(result_len);
    memcpy(result, a.buff, la);
    memcpy(result + la, b.buff, lb);
    result[la + lb] = '\0';

    CrowStr res = {
        .buff = result,
        .len = result_len
    };  

    return res;
}

size_t crow_str_len(CrowStr s) {
    return s.len;
}

int8_t crow_str_eq(CrowStr a, CrowStr b) {
    return strcmp(a.buff, b.buff) == 0;
}