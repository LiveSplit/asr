use crate::runtime::sys;

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
#[derive(Debug)]
#[repr(transparent)]
pub struct Map(pub(super) sys::SettingsMap);

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
}
