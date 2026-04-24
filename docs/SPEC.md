#### 🍹 Examples
```
use std/io
use std/result for Result
use std/option as o

fun greet(name: str) {
  io.println("Hello, " + name)
}

fun main() {
  let name = io.readln()
  greet(name)
}

fun do_nothing() {
  none
}

pub fun hello(n: int) {
  if n > 0 {
    io.println("Hello!")
    hello(n - 1)
  }
}

enum Color {
  Rgb(int, int, int),
  Hex(str)
}

pub enum Flower {
  Dandelion,
  Rose,
  Tulip,
  Lily
}

pub enum Pot {
  Empty,
  Full(Flower)
}

struct Home {
  street: str,
  number: int,
}

pure fun sum(a: int, b: int) -> int {
  a + b
}

fun do_something() {
  #[
    function `sum` is not called, 
    because `sum` is pure and result is ignored (optimization)
  ]#
  let _ = sum(3, 4) 
}

enum Dog {
  Bulldog,
  Dalmatian,
  Husky
}

fun bark(dog: Dog) {
  match dog {
    Bulldog -> "Brr!",
    Dalmatian -> "Woof!",
    Husky -> "Brr! Woof!"
  }
}
```
