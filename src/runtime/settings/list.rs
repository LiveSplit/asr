use core::{borrow::Borrow, fmt};

use crate::{runtime::sys, Error};

use super::{AsValue, Value};

/// A list of [`Value`]s that can itself be a [`Value`] and thus be stored in a
/// [`Map`](super::Map).
#[repr(transparent)]
pub struct List(pub(super) sys::SettingsList);

impl fmt::Debug for List {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl Drop for List {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: The handle is valid and we own it, so it's our responsibility
        // to free it.
        unsafe { sys::settings_list_free(self.0) }
    }
}

impl Clone for List {
    #[inline]
    fn clone(&self) -> Self {
        // SAFETY: The handle is valid, so we can safely copy it.
        Self(unsafe { sys::settings_list_copy(self.0) })
    }
}

impl Default for List {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl List {
    /// Creates a new empty settings list.
    #[inline]
    pub fn new() -> Self {
        // SAFETY: This is always safe to call.
        Self(unsafe { sys::settings_list_new() })
    }

    /// Returns the number of values in the list.
    #[inline]
    pub fn len(&self) -> u64 {
        // SAFETY: The handle is valid, so we can safely call this function.
        unsafe { sys::settings_list_len(self.0) }
    }

    /// Returns [`true`] if the list has a length of 0.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a copy of the value at the given index. Returns [`None`] if the
    /// index is out of bounds. Any changes to it are only perceived if it's
    /// stored back.
    #[inline]
    pub fn get(&self, index: u64) -> Option<Value> {
        // SAFETY: The settings list handle is valid.
        unsafe { sys::settings_list_get(self.0, index).map(Value) }
    }

    /// Pushes a copy of the value to the end of the list.
    #[inline]
    pub fn push(&self, value: impl AsValue) {
        // SAFETY: The settings list handle is valid and the value handle is
        // valid.
        unsafe { sys::settings_list_push(self.0, value.as_value().borrow().0) }
    }

    /// Inserts a copy of the value at the given index, pushing all values at
    /// and after the index one position further. Returns an error if the index
    /// is out of bounds. You may specify an index that is equal to the length
    /// of the list to append the value to the end of the list.
    #[inline]
    pub fn insert(&self, index: u64, value: impl AsValue) -> Result<(), Error> {
        // SAFETY: The settings list handle is valid and the value handle is
        // valid.
        unsafe {
            if sys::settings_list_insert(self.0, index, value.as_value().borrow().0) {
                Ok(())
            } else {
                Err(Error {})
            }
        }
    }

    /// Returns an iterator over the values in the list. Every value is a copy,
    /// so any changes to them are only perceived if they are stored back. The
    /// iterator is double-ended, so it can be iterated backwards as well. While
    /// it's possible to modify the list while iterating over it, it's not
    /// recommended to do so, as the iterator might skip values or return
    /// duplicate values. In that case it's better to clone the list before and
    /// iterate over the clone or use [`get`](Self::get) to manually handle the
    /// iteration.
    #[inline]
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = Value> + '_ {
        (0..self.len()).flat_map(|i| self.get(i))
    }
}
