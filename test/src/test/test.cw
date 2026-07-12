@intrinsic("ptr_read")
fun ptr_read_i32(ptr: raw_ptr, offset: i64) -> i32

@intrinsic("ptr_read")
fun ptr_read_ptr(ptr: raw_ptr, offset: i64) -> raw_ptr

@intrinsic("ptr_write")
fun ptr_write_i32(ptr: raw_ptr, offset: i64, value: i32)

@intrinsic("ptr_write")
fun ptr_write_ptr(ptr: raw_ptr, offset: i64, value: raw_ptr)

@intrinsic("ptr_offset")
fun ptr_offset(ptr: raw_ptr, offset: i64) -> raw_ptr

@intrinsic("str_len")
fun str_len(s: str) -> i64

@intrinsic("str_ptr")
fun str_ptr(s: str) -> raw_ptr

@intrinsic("trap")
fun trap() -> !

native fun exit(code: i64) -> ! = "exit"
native fun malloc(size: i64) -> raw_ptr = "malloc"
native fun free(ptr: raw_ptr) = "free"
native fun print_i32(n: i32) = "crow_print_i32"
native fun write(fd: i64, ptr: raw_ptr, size: i64) -> i64 = "write"


@lang_def("panic")
fun panic(msg: str) -> ! {
    val len = str_len(msg)
    _ = write(1, str_ptr(msg), len)
    exit(-1)
}

@lang_def("rc_retain")
fun rc_retain(ptr: raw_ptr) {
    val header = ptr_offset(ptr, -4)
    val count = ptr_read_i32(header, 0)
    ptr_write_i32(header, 0, count + 1)
}

@lang_def("rc_release")
fun rc_release(ptr: raw_ptr) {
    val header = ptr_offset(ptr, -4)
    val count = ptr_read_i32(header, 0)
    if count == 1 {
        free(header)
    } else {
        ptr_write_i32(header, 0, count - 1)
    }
}

@lang_def("rc_alloc")
fun rc_alloc(size: i64) -> raw_ptr {
    val block = malloc(size + 4)
    ptr_write_i32(block, 0, 1)
    val ptr = ptr_offset(block, 4)
    ptr
}

fun main() -> i8 {
    val a: str = "Hello "
    val b: str = "World!\n"

    val new_str = a ++ b
    val len = str_len(new_str)
    _ = write(1, str_ptr(new_str), len)
    0
}