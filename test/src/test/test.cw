enum Option[T] {
    Some(T),
    None
}

fun sum(a: int, b: int) -> int {
    a + b
}

fun main() {
    let a = Some(Some(3))
    let b = match a {
        .Some(.Some("hello")) -> 1,
        _ -> 2
    }
}
