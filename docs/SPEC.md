### Idea
Crow is expression-based, functional, systems programming language

### Top level items
#### Functions
Without modifiers:
```
fun fib(n: i16): i16 {
  if n <= 1 {
    n
  } else {
    fib(n - 1) + fib(n - 2)
  }
}
```
With `pub` modifier:
```
pub fun sum(a: i16, b: i16) {
  a + b
}
```
With default `priv` modifier:
```
priv fun factorial(n: i64): i64 {
  let m = 1

  for x in 1..n {
    m *= x
  }

  m    
}
```
#### Structs
```
pub struct Apple {
  id: i32,
  kind: [u8; 16],
  price: i8,
}
```
#### Enums
```
priv enum Pillow {
  Soft,
  Raw,
  Hard
}
```
#### Type aliases
Union types:
```
priv type Id = (i16 | i32)
```
Type aliases:
```
pub type int = i64
```
#### Uses / imports
Import of the module:
```
use std/io
```
Import specific items from module:
```
use std/math for sin, cos
```
### Statements
#### Variable declaration
Immutable variable declaration:
```
let a = 5
```
Mutable variable declration:
```
let mut a = 5
```
#### Pointer drop
```
drop ptr
```
### Expressions
#### Unary expressions
```
( - | ! | & | * ) value
```
#### Binary expressions
```
lhs ( + | - | * | / | % | == | != | > | >= | < | <= | & | | | && | || | ^ ) rhs
```
#### If/else expressions
```
let a = if b > 5 {
    0
} else {
    1
}
```
```
if a > 5 {
    ...
}
```
```
if a > 0 {
    ...
} else if a < 0 {
    ...
} else {
    ...
}
```
#### Match expression
```
match int {
  i64 as a => ...
  i32 => ...
  _ => ...
}
```
#### Id expressions
```
variable
```
#### Field expressions
```
a.b.c
```
#### Call expressions
```
a().b.c().d()
```
#### Assign expressions
```
a = 5
b.c.d = 5
e[1] = 0
```
#### Index expressions
```
a[0]
```
#### Alloc expressions
```
alloc 1234
```
#### Array expressions
```
[1, 2, 3, 4, 5, 6, 7, 8]
```
### Semantics
#### Pointers or references
Heap allocation:
```
fun fresh_id(): &Id {
    alloc 123
}
```
Drop data behind a pointer:
```
fun take(a: &Order) {
    drop a
}
```
Null pointers:
```
fun fresh_ptr(): &dyn {
    nil
}
```
Dynamic pointer:
````
let a = alloc 123
let b = a as &dyn
let c = b as &f64
```
