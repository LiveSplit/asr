use core::{borrow::Borrow, fmt, mem::MaybeUninit};

use arrayvec::ArrayString;

use crate::{runtime::sys, Error};

use super::{List, Map};

/// The type of a setting [`Value`].
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValueType {
    /// A [`Map`] of key-value pairs of [`Value`]s.
    Map = 1,
    /// A [`List`] of [`Value`]s.
    List,
    /// A boolean.
    Bool,
    /// A 64-bit signed integer.
    I64,
    /// A 64-bit floating point number.
    F64,
    /// A string.
    String,
    /// The type is not known. This is likely the case if the auto splitter is
    /// running on a newer runtime that supports more types.
    Unknown,
}

/// A value of a setting. This can be a value of any type that a setting can
/// hold. Currently this is either a [`Map`], a [`List`], a [`bool`], an
/// [`i64`], an [`f64`], or a string.
#[repr(transparent)]
pub struct Value(pub(super) sys::SettingValue);

impl fmt::Debug for Value {
    #[allow(clippy::collapsible_match)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get_type() {
            ValueType::Map => {
                if let Some(v) = self.get_map() {
                    return fmt::Debug::fmt(&v, f);
                }
            }
            ValueType::List => {
                if let Some(v) = self.get_list() {
                    return fmt::Debug::fmt(&v, f);
                }
            }
            ValueType::Bool => {
                if let Some(v) = self.get_bool() {
                    return fmt::Debug::fmt(&v, f);
                }
            }
            ValueType::I64 => {
                if let Some(v) = self.get_i64() {
                    return fmt::Debug::fmt(&v, f);
                }
            }
            ValueType::F64 => {
                if let Some(v) = self.get_f64() {
                    return fmt::Debug::fmt(&v, f);
                }
            }
            ValueType::String => {
                if let Some(v) = self.get_array_string::<128>() {
                    if let Ok(v) = v {
                        return fmt::Debug::fmt(&v, f);
                    }
                    #[cfg(not(feature = "alloc"))]
                    return f.write_str("<Long string>");
                }
                #[cfg(feature = "alloc")]
                if let Some(v) = self.get_string() {
                    return fmt::Debug::fmt(&v, f);
                }
            }
            _ => {}
        }
        f.write_str("<Unknown>")
    }
}

impl Clone for Value {
    #[inline]
    fn clone(&self) -> Self {
        // SAFETY: The handle is valid, so we can safely copy it.
        Self(unsafe { sys::setting_value_copy(self.0) })
    }
}

impl Value {
    /// Creates a new setting value from a value of a supported type. The value
    /// is going to be copied inside. Any changes to the original value are not
    /// reflected in the setting value.
    #[inline]
    pub fn new(value: impl Into<Self>) -> Self {
        value.into()
    }

    /// Returns the type of the value.
    #[inline]
    pub fn get_type(&self) -> ValueType {
        // SAFETY: The handle is valid, so we can safely call this function.
        unsafe {
            match sys::setting_value_get_type(self.0) {
                sys::SettingValueType::MAP => ValueType::Map,
                sys::SettingValueType::LIST => ValueType::List,
                sys::SettingValueType::BOOL => ValueType::Bool,
                sys::SettingValueType::I64 => ValueType::I64,
                sys::SettingValueType::F64 => ValueType::F64,
                sys::SettingValueType::STRING => ValueType::String,
                _ => ValueType::Unknown,
            }
        }
    }

    /// Returns the value as a [`Map`] if it is a map. The map is a copy, so any
    /// changes to it are not reflected in the setting value.
    #[inline]
    pub fn get_map(&self) -> Option<Map> {
        // SAFETY: The handle is valid. We provide a valid pointer to a map.
        // After the function call we check the return value and if it's true,
        // the map is initialized and we can return it. We also own the map
        // handle, so it's our responsibility to free it.
        unsafe {
            let mut out = MaybeUninit::uninit();
            if sys::setting_value_get_map(self.0, out.as_mut_ptr()) {
                Some(Map(out.assume_init()))
            } else {
                None
            }
        }
    }

    /// Returns the value as a [`List`] if it is a list. The list is a copy, so
    /// any changes to it are not reflected in the setting value.
    #[inline]
    pub fn get_list(&self) -> Option<List> {
        // SAFETY: The handle is valid. We provide a valid pointer to a list.
        // After the function call we check the return value and if it's true,
        // the list is initialized and we can return it. We also own the list
        // handle, so it's our responsibility to free it.
        unsafe {
            let mut out = MaybeUninit::uninit();
            if sys::setting_value_get_list(self.0, out.as_mut_ptr()) {
                Some(List(out.assume_init()))
            } else {
                None
            }
        }
    }

    /// Returns the value as a [`bool`] if it is a boolean.
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

    /// Returns the value as an [`i64`] if it is a 64-bit signed integer.
    #[inline]
    pub fn get_i64(&self) -> Option<i64> {
        // SAFETY: The handle is valid. We provide a valid pointer to a i64.
        // After the function call we check the return value and if it's true,
        // the i64 is initialized and we can return it.
        unsafe {
            let mut out = MaybeUninit::uninit();
            if sys::setting_value_get_i64(self.0, out.as_mut_ptr()) {
                Some(out.assume_init())
            } else {
                None
            }
        }
    }

    /// Returns the value as an [`f64`] if it is a 64-bit floating point number.
    #[inline]
    pub fn get_f64(&self) -> Option<f64> {
        // SAFETY: The handle is valid. We provide a valid pointer to a f64.
        // After the function call we check the return value and if it's true,
        // the f64 is initialized and we can return it.
        unsafe {
            let mut out = MaybeUninit::uninit();
            if sys::setting_value_get_f64(self.0, out.as_mut_ptr()) {
                Some(out.assume_init())
            } else {
                None
            }
        }
    }

    /// Returns the value as a [`String`](alloc::string::String) if it is a string.
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn get_string(&self) -> Option<alloc::string::String> {
        // SAFETY: The handle is valid. We provide a null pointer and 0 as the
        // length to get the length of the string. If it failed and the length
        // is 0, then that indicates that the value is not a string and we
        // return None. Otherwise we allocate a buffer of the returned length
        // and call the function again with the buffer. This should now always
        // succeed and we can return the string. The function also guarantees
        // that the buffer is valid UTF-8.
        unsafe {
            let mut len = 0;
            let success = sys::setting_value_get_string(self.0, core::ptr::null_mut(), &mut len);
            if len == 0 && !success {
                return None;
            }
            let mut buf = alloc::vec::Vec::with_capacity(len);
            let success = sys::setting_value_get_string(self.0, buf.as_mut_ptr(), &mut len);
            assert!(success);
            buf.set_len(len);
            Some(alloc::string::String::from_utf8_unchecked(buf))
        }
    }

    /// Returns the value as an [`ArrayString`] if it is a string. Returns an
    /// error if the string is too long. The constant `N` determines the maximum
    /// length of the string in bytes.
    #[inline]
    pub fn get_array_string<const N: usize>(&self) -> Option<Result<ArrayString<N>, Error>> {
        // SAFETY: The handle is valid. We provide a pointer to our buffer and
        // the length of the buffer. If the function fails, we check the length
        // and if it's 0, then that indicates that the value is not a string and
        // we return None. Otherwise we return an error. If the function
        // succeeds, we set the length of the buffer to the returned length and
        // return the string. The function also guarantees that the buffer is
        // valid UTF-8.
        unsafe {
            let mut buf = ArrayString::<N>::new();
            let mut len = N;
            let success =
                sys::setting_value_get_string(self.0, buf.as_bytes_mut().as_mut_ptr(), &mut len);
            if !success {
                return if len == 0 { None } else { Some(Err(Error {})) };
            }
            buf.set_len(len);
            Some(Ok(buf))
        }
    }
}

impl Drop for Value {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: The handle is valid and we own it, so it's our responsibility
        // to free it.
        unsafe { sys::setting_value_free(self.0) }
    }
}

/// A trait for types that can be converted into a [`Value`] or allow accessing
/// it as a reference.
pub trait AsValue {
    /// The type of the value. It needs to be able to be borrowed as a
    /// [`Value`].
    type Output: Borrow<Value>;
    /// Converts the value into a type that can be borrowed as a [`Value`].
    #[allow(clippy::wrong_self_convention)]
    fn as_value(self) -> Self::Output;
}

impl<'a> AsValue for &'a Value {
    type Output = Self;
    fn as_value(self) -> Self::Output {
        self
    }
}

impl<T> AsValue for T
where
    Value: From<T>,
{
    type Output = Value;
    fn as_value(self) -> Self::Output {
        self.into()
    }
}

impl From<&Map> for Value {
    #[inline]
    fn from(value: &Map) -> Self {
        // SAFETY: The handle is valid. We retain ownership of the handle, so we
        // only take a reference to a map. We own the returned value now.
        Self(unsafe { sys::setting_value_new_map(value.0) })
    }
}

impl From<&List> for Value {
    #[inline]
    fn from(value: &List) -> Self {
        // SAFETY: The handle is valid. We retain ownership of the handle, so we
        // only take a reference to a List. We own the returned value now.
        Self(unsafe { sys::setting_value_new_list(value.0) })
    }
}

impl From<Map> for Value {
    #[inline]
    fn from(value: Map) -> Self {
        (&value).into()
    }
}

impl From<List> for Value {
    #[inline]
    fn from(value: List) -> Self {
        (&value).into()
    }
}

impl From<bool> for Value {
    #[inline]
    fn from(value: bool) -> Self {
        // SAFETY: This is always safe to call. We own the returned value now.
        Self(unsafe { sys::setting_value_new_bool(value) })
    }
}

impl From<i64> for Value {
    #[inline]
    fn from(value: i64) -> Self {
        // SAFETY: This is always safe to call. We own the returned value now.
        Self(unsafe { sys::setting_value_new_i64(value) })
    }
}

impl From<f64> for Value {
    #[inline]
    fn from(value: f64) -> Self {
        // SAFETY: This is always safe to call. We own the returned value now.
        Self(unsafe { sys::setting_value_new_f64(value) })
    }
}

impl From<&str> for Value {
    #[inline]
    fn from(value: &str) -> Self {
        // SAFETY: We provide a valid pointer and length to the string which is
        // guaranteed to be valid UTF-8. We own the returned value now.
        Self(unsafe { sys::setting_value_new_string(value.as_ptr(), value.len()) })
    }
}
