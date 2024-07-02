//! This module is not Godot specific and instead provides generic utilities for
//! working with processes written in C++. It could be moved outside at some
//! point in the future.

mod ptr;
mod type_info;
mod vtable;

pub use ptr::*;
pub use type_info::*;
pub use vtable::*;
