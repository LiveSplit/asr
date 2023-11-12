use core::fmt;

use arrayvec::ArrayString;

use crate::{runtime::sys, Error};

use super::Value;

/// A map consisting of settings that are configured. Every setting has a string
/// based key and a [`Value`]. There is a global settings map that represents
/// all the settings that the user has configured at the given time. Settings
/// that are unmodified are usually not stored in the map. The global map is
/// what gets stored to disk and loaded from disk. Any changes made in the
/// settings GUI will be reflected in the global map and vice versa. The key of
/// the settings widget is used as the key for the settings map. Additional
/// settings that are not part of the GUI can be stored in the map as well, such
/// as a version of the settings for handling old versions of an auto splitter.
#[repr(transparent)]
pub struct Map(pub(super) sys::SettingsMap);

impl fmt::Debug for Map {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "alloc")]
        let entries = self.iter();
        #[cfg(not(feature = "alloc"))]
        let entries = self.iter_array_string::<128>();
        f.debug_map().entries(entries).finish()
    }
}

impl Drop for Map {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: The handle is valid and we own it, so it's our responsibility
        // to free it.
        unsafe { sys::settings_map_free(self.0) }
    }
}

impl Clone for Map {
    #[inline]
    fn clone(&self) -> Self {
        // SAFETY: The handle is valid, so we can safely copy it.
        Self(unsafe { sys::settings_map_copy(self.0) })
    }
}

impl Default for Map {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Map {
    /// Creates a new empty settings map.
    #[inline]
    pub fn new() -> Self {
        // SAFETY: This is always safe to call.
        Self(unsafe { sys::settings_map_new() })
    }

    /// Loads a copy of the currently set global settings map. Any changes to it
    /// are only perceived if it's stored back.
    #[inline]
    pub fn load() -> Self {
        // SAFETY: This is always safe to call.
        Self(unsafe { sys::settings_map_load() })
    }

    /// Stores a copy of the settings map as the new global settings map. This
    /// will overwrite the previous global settings map. There's a chance that
    /// the settings map was changed in the meantime, so those changes could get
    /// lost. Prefer using [`store_if_unchanged`](Self::store_if_unchanged) if
    /// you want to avoid that.
    #[inline]
    pub fn store(&self) {
        // SAFETY: The handle is valid, so it can be stored.
        unsafe { sys::settings_map_store(self.0) }
    }

    /// Stores a copy of the new settings map as the new global settings map if
    /// the map has not changed in the meantime. This is done by comparing the
    /// old map. Returns [`true`] if the map was stored successfully.
    /// Returns [`false`] if the map was changed in the meantime.
    #[inline]
    pub fn store_if_unchanged(&self, old: &Self) -> bool {
        // SAFETY: Both handles are valid.
        unsafe { sys::settings_map_store_if_unchanged(old.0, self.0) }
    }

    /// Inserts a copy of the setting value into the settings map based on the
    /// key. If the key already exists, it will be overwritten.
    #[inline]
    pub fn insert(&self, key: &str, value: &Value) {
        // SAFETY: The settings map handle is valid. We provide a valid pointer
        // and length to the key which is guaranteed to be valid UTF-8. The
        // setting value handle is also valid.
        unsafe { sys::settings_map_insert(self.0, key.as_ptr(), key.len(), value.0) }
    }

    /// Gets a copy of the setting value from the settings map based on the key.
    /// Returns [`None`] if the key does not exist. Any changes to it are only
    /// perceived if it's stored back.
    #[inline]
    pub fn get(&self, key: &str) -> Option<Value> {
        // SAFETY: The settings map handle is valid. We provide a valid pointer
        // and length to the key which is guaranteed to be valid UTF-8.
        unsafe { sys::settings_map_get(self.0, key.as_ptr(), key.len()).map(Value) }
    }

    /// Returns the number of key value pairs in the map.
    #[inline]
    pub fn len(&self) -> u64 {
        // SAFETY: The handle is valid, so we can safely call this function.
        unsafe { sys::settings_map_len(self.0) }
    }

    /// Returns [`true`] if the map has a length of 0.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the key at the given index. Returns [`None`] if the index is out
    /// of bounds.
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn get_key_by_index(&self, index: u64) -> Option<alloc::string::String> {
        // SAFETY: The handle is valid. We provide a null pointer and 0 as the
        // length to get the length of the string. If it failed and the length
        // is 0, then that indicates that the index is out of bounds and we
        // return None. Otherwise we allocate a buffer of the returned length
        // and call the function again with the buffer. This should now always
        // succeed and we can return the string. The function also guarantees
        // that the buffer is valid UTF-8.
        unsafe {
            let mut len = 0;
            let success =
                sys::settings_map_get_key_by_index(self.0, index, core::ptr::null_mut(), &mut len);
            if len == 0 && !success {
                return None;
            }
            let mut buf = alloc::vec::Vec::with_capacity(len);
            let success =
                sys::settings_map_get_key_by_index(self.0, index, buf.as_mut_ptr(), &mut len);
            assert!(success);
            buf.set_len(len);
            Some(alloc::string::String::from_utf8_unchecked(buf))
        }
    }

    /// Returns the key at the given index as an [`ArrayString`]. Returns
    /// [`None`] if the index is out of bounds. Returns an error if the key is
    /// does not fit into the array string.
    #[inline]
    pub fn get_key_by_index_array_string<const N: usize>(
        &self,
        index: u64,
    ) -> Option<Result<ArrayString<N>, Error>> {
        // SAFETY: The handle is valid. We provide a pointer to our buffer and
        // the length of the buffer. If the function fails, we check the length
        // and if it's 0, then that indicates that the index is out of bounds
        // and we return None. Otherwise we return an error. If the function
        // succeeds, we set the length of the buffer to the returned length and
        // return the string. The function also guarantees that the buffer is
        // valid UTF-8.
        unsafe {
            let mut buf = ArrayString::<N>::new();
            let mut len = N;
            let success = sys::settings_map_get_key_by_index(
                self.0,
                index,
                buf.as_bytes_mut().as_mut_ptr(),
                &mut len,
            );
            if !success {
                return if len == 0 { None } else { Some(Err(Error {})) };
            }
            buf.set_len(len);
            Some(Ok(buf))
        }
    }

    /// Returns a copy of the value at the given index. Returns [`None`] if the index is
    /// out of bounds. Any changes to it are only perceived if it's stored back.
    #[inline]
    pub fn get_value_by_index(&self, index: u64) -> Option<Value> {
        // SAFETY: The settings map handle is valid. We do proper error handling
        // afterwards.
        unsafe { sys::settings_map_get_value_by_index(self.0, index).map(Value) }
    }

    /// Returns an iterator over the key value pairs of the map. Every value is
    /// a copy, so any changes to them are only perceived if they are stored
    /// back. The iterator is double-ended, so it can be iterated backwards as
    /// well. While it's possible to modify the map while iterating over it,
    /// it's not recommended to do so, as the iterator might skip pairs or
    /// return duplicates. In that case it's better to clone the map before and
    /// iterate over the clone.
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (alloc::string::String, Value)> + '_ {
        (0..self.len()).flat_map(|i| Some((self.get_key_by_index(i)?, self.get_value_by_index(i)?)))
    }

    /// Returns an iterator over the key value pairs of the map. The keys are
    /// returned as [`ArrayString`]. The iterator yields an error for every key
    /// that does not fit into the array string. Every value is a copy, so any
    /// changes to them are only perceived if they are stored back. The iterator is
    /// double-ended, so it can be iterated backwards as well. While it's
    /// possible to modify the map while iterating over it, it's not recommended
    /// to do so, as the iterator might skip pairs or return duplicates. In that
    /// case it's better to clone the map before and iterate over the clone.
    #[inline]
    pub fn iter_array_string<const N: usize>(
        &self,
    ) -> impl DoubleEndedIterator<Item = (Result<ArrayString<N>, Error>, Value)> + '_ {
        (0..self.len()).flat_map(|i| {
            Some((
                self.get_key_by_index_array_string::<N>(i)?,
                self.get_value_by_index(i)?,
            ))
        })
    }

    /// Returns an iterator over the keys of the map. The iterator is
    /// double-ended, so it can be iterated backwards as well. While it's
    /// possible to modify the map while iterating over it, it's not recommended
    /// to do so, as the iterator might skip keys or return duplicates. In that
    /// case it's better to clone the map before and iterate over the clone.s
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn keys(&self) -> impl DoubleEndedIterator<Item = alloc::string::String> + '_ {
        (0..self.len()).flat_map(|i| self.get_key_by_index(i))
    }

    /// Returns an iterator over the keys of the map. The keys are returned as
    /// [`ArrayString`]. The iterator yields an error for every key that does
    /// not fit into the array string. The iterator is double-ended, so it can
    /// be iterated backwards as well. While it's possible to modify the map
    /// while iterating over it, it's not recommended to do so, as the iterator
    /// might skip keys or return duplicates. In that case it's better to clone
    /// the map before and iterate over the clone.
    #[inline]
    pub fn keys_array_string<const N: usize>(
        &self,
    ) -> impl DoubleEndedIterator<Item = Result<ArrayString<N>, Error>> + '_ {
        (0..self.len()).flat_map(|i| self.get_key_by_index_array_string(i))
    }

    /// Returns an iterator over the values of the map. Every value is a copy,
    /// so any changes to them are only perceived if they are stored back. The
    /// iterator is double-ended, so it can be iterated backwards as well. While
    /// it's possible to modify the map while iterating over it, it's not
    /// recommended to do so, as the iterator might skip values or return
    /// duplicate values. In that case it's better to clone the map before and
    /// iterate over the clone.
    #[inline]
    pub fn values(&self) -> impl DoubleEndedIterator<Item = Value> + '_ {
        (0..self.len()).flat_map(|i| self.get_value_by_index(i))
    }
}
