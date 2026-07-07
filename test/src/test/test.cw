native fun print_i8(a: i8) = "crow_print_i8"
native fun print_i32(a: i32) = "crow_print_i32"
native fun print_str(a: string) = "crow_print_str"
native fun println() = "crow_println"
native fun read_line() -> string = "crow_read_line"
native fun read_i32() -> i32 = "crow_read_i32"
native fun read_i64() -> i64 = "crow_read_i64"
native fun read_f64() -> f64 = "crow_read_f64"

fun main() -> i8 {
    print_str("Enter your name: ")
    val name = read_line()
    print_str("Hello, ")
    print_str(name)
    print_str("!\n")
    print_str("Enter a number: ")
    val n = read_i32()
    print_i32(n * 2)
    println()
    0
}