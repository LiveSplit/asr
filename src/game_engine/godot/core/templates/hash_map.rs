//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/core/templates/hash_map.h>

use core::{iter, mem::size_of};

use crate::{game_engine::godot::Ptr, Address64, Error, Process};

/// A type that we know the size of in the target process.
pub trait KnownSize {}

/// A hash map that maps keys to values. This is not publicly exposed as such in
/// Godot, because it's a template class. The closest equivalent is the general
/// [`Dictionary`](https://docs.godotengine.org/en/4.2/classes/class_dictionary.html).
///
/// Check the [`Ptr`] documentation to see all the methods you can call on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct HashMap<K, V>(core::marker::PhantomData<(K, V)>);

impl<K, V> Ptr<HashMap<K, V>> {
    /// Returns an iterator over the key-value pairs in this hash map.
    pub fn iter<'a>(&'a self, process: &'a Process) -> impl Iterator<Item = (Ptr<K>, Ptr<V>)> + 'a
    where
        K: KnownSize,
    {
        let mut current: Address64 = self.read_at_offset(0x18, process).unwrap_or_default();
        iter::from_fn(move || {
            if current.is_null() {
                return None;
            }
            let ret = (
                Ptr::new(current + 0x10),
                Ptr::new(current + 0x10 + size_of::<K>() as u64),
            );
            current = process.read(current).ok()?;
            Some(ret)
        })
    }

    /// Returns the number of elements in this hash map.
    pub fn size(self, process: &Process) -> Result<u32, Error> {
        self.read_at_offset(0x2C, process)
    }
}
