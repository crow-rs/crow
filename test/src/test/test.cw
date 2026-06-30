fun meme(a: i8, b: i8) -> i8 {
    a + b
}

fun main() -> i8 {
    let a: i8 = meme(1, 2)
    let c: i8 = 100 + 100

    match a {
        4 -> 20,
        _ -> 10
    }
}
