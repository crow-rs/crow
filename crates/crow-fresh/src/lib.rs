/// Imports
use num_traits::PrimInt;
use std::ops::Add;

/// Defines freshen variable
pub struct Freshen<T: PrimInt + Add> {
    next_id: T,
}

/// Implementation
impl<T: PrimInt + Add> Freshen<T> {
    /// Creates new freshen
    pub fn new() -> Self {
        Self { next_id: T::zero() }
    }

    /// Returns fresh id
    pub fn fresh(&mut self) -> T {
        self.next_id = self.next_id.add(T::one());
        self.next_id.sub(T::one())
    }
}

/// Defines freshen vector
#[derive(Debug, Clone)]
pub struct FreshenVec<N: PrimInt + Add, T> {
    next_id: N,
    vec: Vec<T>,
}

/// Implementation
impl<N: PrimInt + Add, T> FreshenVec<N, T> {
    /// Creates new freshen vec
    pub fn new() -> Self {
        Self {
            next_id: N::zero(),
            vec: Vec::new(),
        }
    }

    /// Allocates new `T`
    pub fn alloc(&mut self, t: T) -> N {
        self.vec.push(t);
        self.next_id = self.next_id.add(N::one());
        self.next_id.sub(N::one())
    }

    /// Returns `T` at `N`
    pub fn item_at(&self, n: N) -> &T {
        &self.vec[n
            .to_usize()
            .expect("`N` should always return valid usize in `to_usize`")]
    }

    /// Returns next id
    pub fn next_id(&self) -> N {
        self.next_id
    }

    /// Takes `Vec<T>` using `std::mem::take()`
    pub fn take(&mut self) -> Self {
        Self {
            next_id: self.next_id,
            vec: std::mem::take(&mut self.vec),
        }
    }

    /// Replaces `Vec<T>` and `next_id` with new ones, returns old value
    pub fn replace(&mut self, target: Self) -> Self {
        let old_next_id = self.next_id;
        let old_vec = std::mem::replace(&mut self.vec, target.vec);
        self.next_id = target.next_id;

        Self {
            vec: old_vec,
            next_id: old_next_id,
        }
    }

    /// Returns internal `Vec<T>`
    pub fn vec(&self) -> &Vec<T> {
        &self.vec
    }
}
