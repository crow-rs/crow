use std/io
use std/result for Result
use std/option as o
use std/list

fun greet(name: str) {
  io.println("Hello, " + name)
}

fun main() {
  let name = io.readln()
  greet(name)
}

fun do_nothing() {
  # none is just a unit value
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
    because `sum` is pure and result is ignored (compiler will optimize it)
  ]#
  let _ = sum(3, 4)
}

enum Dog {
  Bulldog,
  Dalmatian,
  Husky
}

fun bark(dog: Dog) -> str {
  match dog {
    .Bulldog -> "Brr!",
    .Dalmatian -> "Woof!",
    .Husky -> "Brr! Woof!"
  }
}

fun test() {
  let a = List()
  list.push(a, 123)
  list.push(a, 321)
  a = list.map(a, fun(a: int) -> a + 1)
}

fun test2() {
  print("Hello, world!")
}

fun unimpl() {
  todo as "no implementation yet"
}

fun error() {
  panic as "panic occurred"
}
