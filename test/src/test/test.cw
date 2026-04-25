enum Option[T] {
    Some(T),
    None
}

fun sum(a: int, b: int) -> int {
    a + b
}

fun main() {
    let a = Some(3)
    a = Some(true)
    a = 3
    let b = sum("hello", 1)
}
