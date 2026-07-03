fun meme(a: i8, b: i8) -> i8 {
    a + b
}

fun main() -> i8 {
    _ = meme(1, 1)

    meme(1, 1)

    val a: i8 = meme(1, 2)
    val c: i8 = 100 + 100
    var d = 10
    match a {
        4 -> 20,
        val b -> 10
    }
}
