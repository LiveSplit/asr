//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/core/templates/vector.h>

use core::mem::size_of;

use bytemuck::{Pod, Zeroable};

use crate::game_engine::godot::{Ptr, SizeInTargetProcess};

use super::CowData;

/// A contiguous vector of elements. This is not publicly exposed in Godot.
#[repr(C)]
pub struct Vector<T> {
    // lol this is pure padding, they messed up
    write_proxy: [u8; 0x8],
    cowdata: CowData<T>,
}

impl<T> SizeInTargetProcess for Vector<T> {
    const SIZE: u64 = size_of::<Vector<T>>() as u64;
}

impl<T> Copy for Vector<T> {}

impl<T> Clone for Vector<T> {
    fn clone(&self) -> Self {
        *self
    }
}

// SAFETY: The type is transparent over a `CowData` and a byte array, which is `Pod`.
unsafe impl<T: 'static> Pod for Vector<T> {}

// SAFETY: The type is transparent over a `CowData` and a byte array, which is `Zeroable`.
unsafe impl<T> Zeroable for Vector<T> {}

impl<T: SizeInTargetProcess> Vector<T> {
    /// Returns the pointer to the underlying data at the given index. This does
    /// not perform bounds checking.
    pub fn unchecked_at(&self, index: u64) -> Ptr<T> {
        Ptr::new(self.cowdata.ptr().addr() + index.wrapping_mul(T::SIZE))
    }
}
