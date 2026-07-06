rec Meme {
    a: i32,
    b: i8
}

fun main() -> i8 {
    val test = Meme {
        a = 10,
        b = 40
    }

    match test.a {
        4 -> 20,
        10 -> 13
    }
}
