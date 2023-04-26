use core::{ops, str};

use bytemuck::{Pod, Zeroable};

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct ArrayCString<const N: usize>([u8; N]);

impl<const N: usize> ArrayCString<N> {
    pub const fn new() -> Self {
        Self([0; N])
    }

    pub fn as_bytes(&self) -> &[u8] {
        let len = self.0.iter().position(|&b| b == 0).unwrap_or(N);
        &self.0[..len]
    }

    pub fn validate_utf8(&self) -> Result<&str, str::Utf8Error> {
        str::from_utf8(self.as_bytes())
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

unsafe impl<const N: usize> Pod for ArrayCString<N> {}
unsafe impl<const N: usize> Zeroable for ArrayCString<N> {}
