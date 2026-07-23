@intrinsic("ptr_read")
fun ptr_read_i8(ptr: raw_ptr, offset: i64) -> i8
@intrinsic("ptr_read")
fun ptr_read_i16(ptr: raw_ptr, offset: i64) -> i16
@intrinsic("ptr_read")
fun ptr_read_i32(ptr: raw_ptr, offset: i64) -> i32
@intrinsic("ptr_read")
fun ptr_read_i64(ptr: raw_ptr, offset: i64) -> i64
@intrinsic("ptr_read")
fun ptr_read_f32(ptr: raw_ptr, offset: i64) -> f32
@intrinsic("ptr_read")
fun ptr_read_f64(ptr: raw_ptr, offset: i64) -> f64
@intrinsic("ptr_read")
fun ptr_read_ptr(ptr: raw_ptr, offset: i64) -> raw_ptr

@intrinsic("ptr_write")
fun ptr_write_i8(ptr: raw_ptr, offset: i64, value: i8)
@intrinsic("ptr_write")
fun ptr_write_i16(ptr: raw_ptr, offset: i64, value: i16)
@intrinsic("ptr_write")
fun ptr_write_i32(ptr: raw_ptr, offset: i64, value: i32)
@intrinsic("ptr_write")
fun ptr_write_i64(ptr: raw_ptr, offset: i64, value: i64)
@intrinsic("ptr_write")
fun ptr_write_f32(ptr: raw_ptr, offset: i64, value: f32)
@intrinsic("ptr_write")
fun ptr_write_f64(ptr: raw_ptr, offset: i64, value: f64)
@intrinsic("ptr_write")
fun ptr_write_ptr(ptr: raw_ptr, offset: i64, value: raw_ptr)

@intrinsic("ptr_offset")
fun ptr_offset(ptr: raw_ptr, offset: i64) -> raw_ptr

@intrinsic("ptr_copy")
fun ptr_copy(src: raw_ptr, dst: raw_ptr, len: i64)

@intrinsic("str_len")
fun str_len(s: str) -> i64

@intrinsic("str_ptr")
fun str_ptr(s: str) -> raw_ptr

@intrinsic("str_from_parts")
fun str_from_parts(ptr: raw_ptr, len: i64) -> str

@intrinsic("trap")
fun trap() -> !

native fun malloc(size: i64) -> raw_ptr = "malloc"
native fun realloc(ptr: raw_ptr, size: i64) -> raw_ptr = "realloc"
native fun free(ptr: raw_ptr) = "free"

native fun write(fd: i32, ptr: raw_ptr, size: i64) -> i64 = "write"
native fun read(fd: i32, ptr: raw_ptr, size: i64) -> i64 = "read"

native fun exit(code: i32) -> ! = "exit"

@lang_def("panic")
fun panic(msg: str) -> ! {
    _ = write(2, str_ptr(msg), str_len(msg))
    _ = write(2, str_ptr("\n"), 1)
    exit(1)
}

@lang_def("rc_alloc")
fun rc_alloc(size: i64) -> raw_ptr {
    val block = malloc(size + 4)
    ptr_write_i32(block, 0, 1)
    ptr_offset(block, 4)
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

pub fun print(s: str) {
    _ = write(1, str_ptr(s), str_len(s))
}

pub fun println(s: str) {
    print(s)
    print("\n")
}

pub fun eprint(s: str) {
    _ = write(2, str_ptr(s), str_len(s))
}

pub fun eprintln(s: str) {
    eprint(s)
    eprint("\n")
}

pub fun print_uint(n: i64) {
    if n >= 10 {
        print_uint(n / 10)
    }
    val digit = ((n % 10) + 48) as i8
    val buf = malloc(1)
    ptr_write_i8(buf, 0, digit)
    _ = write(1, buf, 1)
    free(buf)
}

pub fun print_int(n: i64) {
    if n < 0 {
        print("-")
        print_uint(0 - n)
    } else {
        print_uint(n)
    }
}

pub fun print_i8(n: i8) {
    print_int(n as i64)
}

pub fun print_i32(n: i32) {
    print_int(n as i64)
}

pub fun print_i64(n: i64) {
    print_int(n)
}

pub fun unreachable() -> ! {
    panic("entered unreachable code")
}

pub fun read_byte() -> i8 {
    val buf = malloc(1)
    _ = read(0, buf, 1)
    val byte = ptr_read_i8(buf, 0)
    free(buf)
    byte
}
