

fun sum(a: int, b: int): tot -> int {
    a + b
}

fun main(): tot -> int {
    let a = sum(10, 20)

    let c = match a {
        10 -> 0,
        30 -> 67,
        _ -> 1337
    }

    c
}
