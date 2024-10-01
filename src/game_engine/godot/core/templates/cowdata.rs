//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/core/templates/cowdata.h>

use bytemuck::{Pod, Zeroable};

use crate::game_engine::godot::Ptr;

/// A copy-on-write data type. This is not publicly exposed in Godot.
#[repr(transparent)]
pub struct CowData<T>(Ptr<T>);

impl<T> Copy for CowData<T> {}

impl<T> Clone for CowData<T> {
    fn clone(&self) -> Self {
        *self
    }
}

// SAFETY: The type is transparent over a `Ptr`, which is `Pod`.
unsafe impl<T: 'static> Pod for CowData<T> {}

// SAFETY: The type is transparent over a `Ptr`, which is `Zeroable`.
unsafe impl<T> Zeroable for CowData<T> {}

impl<T> CowData<T> {
    /// Returns the pointer to the underlying data.
    pub fn ptr(self) -> Ptr<T> {
        self.0
    }
}
