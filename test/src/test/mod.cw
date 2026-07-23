use another_module

fun main() -> i8 {
    println("Hello world!")
    0
}

@lang_def("panic")
fun panic(msg: str) -> ! {
    _ = write(2, str_ptr(msg), str_len(msg))
    _ = write(2, str_ptr("\n"), 1)
    exit(1)
}
