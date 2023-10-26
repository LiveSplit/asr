use core::mem::MaybeUninit;

use crate::runtime::sys;

/// A value of a setting. This can be a value of any type that a setting can
/// hold. Currently only boolean settings are supported.
#[derive(Debug)]
#[repr(transparent)]
pub struct Value(pub(super) sys::SettingValue);

impl Drop for Value {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: The handle is valid and we own it, so it's our responsibility
        // to free it.
        unsafe { sys::setting_value_free(self.0) }
    }
}

impl Value {
    /// Creates a new setting value from a value of a supported type.
    #[inline]
    pub fn new(value: impl Into<Self>) -> Self {
        value.into()
    }

    /// Returns the value as a boolean if it is a boolean.
    #[inline]
    pub fn get_bool(&self) -> Option<bool> {
        // SAFETY: The handle is valid. We provide a valid pointer to a boolean.
        // After the function call we check the return value and if it's true,
        // the boolean is initialized and we can return it.
        unsafe {
            let mut out = MaybeUninit::uninit();
            if sys::setting_value_get_bool(self.0, out.as_mut_ptr()) {
                Some(out.assume_init())
            } else {
                None
            }
        }
    }
}

impl From<bool> for Value {
    #[inline]
    fn from(value: bool) -> Self {
        // SAFETY: This is always safe to call.
        Self(unsafe { sys::setting_value_new_bool(value) })
    }
}
