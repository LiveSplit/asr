//! Support for string types that can be read from a process's memory.

use core::{ops, str};

use bytemuck::{Pod, Zeroable};

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
        <[u8]>::eq(self, &**other)
    }
}

impl<const N: usize> Eq for ArrayCString<N> {}

/// SAFETY: The type is transparent over an array of `N` bytes, which is `Pod`.
unsafe impl<const N: usize> Pod for ArrayCString<N> {}
/// SAFETY: The type is transparent over an array of `N` bytes, which is `Zeroable`.
unsafe impl<const N: usize> Zeroable for ArrayCString<N> {}
