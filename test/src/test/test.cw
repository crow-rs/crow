fun meme(a: i8, b: i8) -> i8 {
    a + b
}

fun main() -> i8 {
    let a: i8 = meme(1, 2)

    match a {
        4 -> 1337,
        _ -> 0
    }
}
