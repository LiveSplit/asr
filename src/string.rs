//! Support for string types that can be read from a process's memory.

use core::{ops, str};

use bytemuck::{Pod, Zeroable};

pub use arrayvec::ArrayString;

use crate::FromEndian;

/// A nul-terminated string that is stored in an array of a fixed size `N`. This
/// can be read from a process's memory.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct ArrayCString<const N: usize>([u8; N]);

impl<const N: usize> ArrayCString<N> {
    /// Creates a new empty nul-terminated string.
    pub const fn new() -> Self {
        Self([0; N])
    }

    /// Returns the bytes of the string up until (but excluding) the
    /// nul-terminator. If there is no nul-terminator, all bytes are returned.
    pub fn as_bytes(&self) -> &[u8] {
        let len = self.0.iter().position(|&b| b == 0).unwrap_or(N);
        &self.0[..len]
    }

    /// Returns the string as a string slice if it is valid UTF-8.
    pub fn validate_utf8(&self) -> Result<&str, str::Utf8Error> {
        str::from_utf8(self.as_bytes())
    }

    /// Checks whether the string matches the given text. This is faster than
    /// calling [`as_bytes`](Self::as_bytes) and then comparing, because it can
    /// use the length information of the parameter.
    pub fn matches(&self, text: impl AsRef<[u8]>) -> bool {
        let bytes = text.as_ref();
        !self.0.get(bytes.len()).is_some_and(|&b| b != 0)
            && self.0.get(..bytes.len()).is_some_and(|s| s == bytes)
    }
}

impl<const N: usize> Default for ArrayCString<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> ops::Deref for ArrayCString<N> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

impl<const N: usize> PartialEq for ArrayCString<N> {
    fn eq(&self, other: &Self) -> bool {
        self.matches(&**other)
    }
}

impl<const N: usize> Eq for ArrayCString<N> {}

/// SAFETY: The type is transparent over an array of `N` bytes, which is `Pod`.
unsafe impl<const N: usize> Pod for ArrayCString<N> {}
/// SAFETY: The type is transparent over an array of `N` bytes, which is `Zeroable`.
unsafe impl<const N: usize> Zeroable for ArrayCString<N> {}

impl<const N: usize> FromEndian for ArrayCString<N> {
    fn from_be(&self) -> Self {
        *self
    }
    fn from_le(&self) -> Self {
        *self
    }
}

/// A nul-terminated wide string (16-bit characters) that is stored in an array
/// of a fixed size of `N` characters. This can be read from a process's memory.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct ArrayWString<const N: usize>([u16; N]);

impl<const N: usize> ArrayWString<N> {
    /// Creates a new empty nul-terminated wide string.
    pub const fn new() -> Self {
        Self([0; N])
    }

    /// Returns the 16-bit characters of the string up until (but excluding) the
    /// nul-terminator. If there is no nul-terminator, all bytes are returned.
    pub fn as_slice(&self) -> &[u16] {
        let len = self.0.iter().position(|&b| b == 0).unwrap_or(N);
        &self.0[..len]
    }

    /// Checks whether the string matches the given text. This is faster than
    /// calling [`as_slice`](Self::as_slice) and then comparing, because it can
    /// use the length information of the parameter.
    pub fn matches(&self, text: impl AsRef<[u16]>) -> bool {
        let bytes = text.as_ref();
        !self.0.get(bytes.len()).is_some_and(|&b| b != 0)
            && self.0.get(..bytes.len()).is_some_and(|s| s == bytes)
    }

    /// Checks whether the string matches the given text. This dynamically
    /// re-encodes the passed in text to UTF-16, which is not as fast as
    /// [`matches`](Self::matches).
    pub fn matches_str(&self, text: &str) -> bool {
        self.as_slice().iter().copied().eq(text.encode_utf16())
    }
}

impl<const N: usize> Default for ArrayWString<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> ops::Deref for ArrayWString<N> {
    type Target = [u16];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<const N: usize> PartialEq for ArrayWString<N> {
    fn eq(&self, other: &Self) -> bool {
        self.matches(&**other)
    }
}

impl<const N: usize> Eq for ArrayWString<N> {}

/// SAFETY: The type is transparent over an array of `N` u16s, which is `Pod`.
unsafe impl<const N: usize> Pod for ArrayWString<N> {}
/// SAFETY: The type is transparent over an array of `N` u16s, which is `Zeroable`.
unsafe impl<const N: usize> Zeroable for ArrayWString<N> {}

impl<const N: usize> FromEndian for ArrayWString<N> {
    fn from_be(&self) -> Self {
        Self(self.0.map(|x| x.from_be()))
    }
    fn from_le(&self) -> Self {
        Self(self.0.map(|x| x.from_le()))
    }
}
