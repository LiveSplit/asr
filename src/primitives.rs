//! Defines integer and floating point number types for reading from big and little
//! endian processes. Unlike the native integer types, these types are unaligned.

use core::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
};

use bytemuck::{Pod, Zeroable};

macro_rules! define_int {
    (#[$name_in_doc:meta] $name:ident => $inner:ident, $to:ident, $from:ident) => {
        #[derive(Copy, Clone, Eq, PartialEq)]
        #[repr(transparent)]
        /// A
        #[$name_in_doc]
        /// integer that can be read from a process's memory. Unlike the native
        /// integer types, these types are unaligned.
        pub struct $name([u8; core::mem::size_of::<$inner>()]);

        impl fmt::Debug for $name {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self.get(), f)
            }
        }

        impl fmt::Display for $name {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.get(), f)
            }
        }

        impl PartialOrd for $name {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                self.get().partial_cmp(&other.get())
            }
        }

        impl Ord for $name {
            #[inline]
            fn cmp(&self, other: &Self) -> Ordering {
                self.get().cmp(&other.get())
            }
        }

        impl Hash for $name {
            #[inline]
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.get().hash(state);
            }
        }

        impl $name {
            /// Creates a new
            #[$name_in_doc]
            /// integer from the given value.
            #[inline]
            pub const fn new(value: $inner) -> Self {
                Self(value.$to())
            }

            /// Returns the underlying integer.
            #[inline]
            pub const fn get(self) -> $inner {
                $inner::$from(self.0)
            }
        }

        // SAFETY: The type is transparent over an array of bytes, which is `Pod`.
        unsafe impl Pod for $name {}
        // SAFETY: The type is transparent over an array of bytes, which is `Zeroable`.
        unsafe impl Zeroable for $name {}
    };
}

macro_rules! define_float {
    (#[$name_in_doc:meta] $name:ident => $inner:ident, $to:ident, $from:ident) => {
        #[derive(Copy, Clone)]
        #[repr(transparent)]
        /// A
        #[$name_in_doc]
        /// floating point number that can be read from a process's memory.
        /// Unlike the native floating point number types, these types are
        /// unaligned.
        pub struct $name([u8; core::mem::size_of::<$inner>()]);

        impl fmt::Debug for $name {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self.get(), f)
            }
        }

        impl fmt::Display for $name {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.get(), f)
            }
        }

        impl PartialEq for $name {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.get().eq(&other.get())
            }
        }

        impl PartialOrd for $name {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                self.get().partial_cmp(&other.get())
            }
        }

        impl $name {
            /// Creates a new
            #[$name_in_doc]
            /// floating point number from the given value.
            #[inline]
            pub fn new(value: $inner) -> Self {
                Self(value.$to())
            }

            /// Returns the underlying floating point number.
            #[inline]
            pub fn get(self) -> $inner {
                $inner::$from(self.0)
            }
        }

        // SAFETY: The type is transparent over an array of bytes, which is `Pod`.
        unsafe impl Pod for $name {}
        // SAFETY: The type is transparent over an array of bytes, which is `Zeroable`.
        unsafe impl Zeroable for $name {}
    };
}

/// Big endian integers and floating point numbers.
pub mod big_endian {
    use super::*;

    define_int!(#[doc = "big endian 16-bit unsigned"] U16 => u16, to_be_bytes, from_be_bytes);
    define_int!(#[doc = "big endian 32-bit unsigned"] U32 => u32, to_be_bytes, from_be_bytes);
    define_int!(#[doc = "big endian 64-bit unsigned"] U64 => u64, to_be_bytes, from_be_bytes);
    define_int!(#[doc = "big endian 128-bit unsigned"] U128 => u128, to_be_bytes, from_be_bytes);

    define_int!(#[doc = "big endian 16-bit signed"] I16 => i16, to_be_bytes, from_be_bytes);
    define_int!(#[doc = "big endian 32-bit signed"] I32 => i32, to_be_bytes, from_be_bytes);
    define_int!(#[doc = "big endian 64-bit signed"] I64 => i64, to_be_bytes, from_be_bytes);
    define_int!(#[doc = "big endian 128-bit signed"] I128 => i128, to_be_bytes, from_be_bytes);

    define_float!(#[doc = "big endian 32-bit"] F32 => f32, to_be_bytes, from_be_bytes);
    define_float!(#[doc = "big endian 64-bit"] F64 => f64, to_be_bytes, from_be_bytes);
}

/// Little endian integers and floating point numbers.
pub mod little_endian {
    use super::*;

    define_int!(#[doc = "little endian 16-bit unsigned"] U16 => u16, to_le_bytes, from_le_bytes);
    define_int!(#[doc = "little endian 32-bit unsigned"] U32 => u32, to_le_bytes, from_le_bytes);
    define_int!(#[doc = "little endian 64-bit unsigned"] U64 => u64, to_le_bytes, from_le_bytes);
    define_int!(#[doc = "little endian 128-bit unsigned"] U128 => u128, to_le_bytes, from_le_bytes);

    define_int!(#[doc = "little endian 16-bit signed"] I16 => i16, to_le_bytes, from_le_bytes);
    define_int!(#[doc = "little endian 32-bit signed"] I32 => i32, to_le_bytes, from_le_bytes);
    define_int!(#[doc = "little endian 64-bit signed"] I64 => i64, to_le_bytes, from_le_bytes);
    define_int!(#[doc = "little endian 128-bit signed"] I128 => i128, to_le_bytes, from_le_bytes);

    define_float!(#[doc = "little endian 32-bit"] F32 => f32, to_le_bytes, from_le_bytes);
    define_float!(#[doc = "little endian 64-bit"] F64 => f64, to_le_bytes, from_le_bytes);
}

/// Support for converting from big or little endian dynamically.
pub mod dynamic_endian {
    /// The endianness of a value.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub enum Endian {
        /// Big endian.
        Big,
        /// Little endian.
        Little,
    }

    use core::array;

    #[cfg(feature = "derive")]
    pub use asr_derive::FromEndian;

    /// A trait for converting from big or little endian.
    #[allow(clippy::wrong_self_convention)]
    pub trait FromEndian: Sized {
        /// Converts the value from big endian.
        fn from_be(&self) -> Self;
        /// Converts the value from little endian.
        fn from_le(&self) -> Self;
        /// Converts the value from the given endian.
        fn from_endian(&self, endian: Endian) -> Self {
            match endian {
                Endian::Big => self.from_be(),
                Endian::Little => self.from_le(),
            }
        }
    }

    macro_rules! define {
        ($name:ident) => {
            impl FromEndian for $name {
                fn from_be(&self) -> Self {
                    $name::from_be_bytes(bytemuck::cast(*self))
                }
                fn from_le(&self) -> Self {
                    $name::from_le_bytes(bytemuck::cast(*self))
                }
            }
        };
    }

    define!(u8);
    define!(u16);
    define!(u32);
    define!(u64);
    define!(u128);
    define!(i8);
    define!(i16);
    define!(i32);
    define!(i64);
    define!(i128);
    define!(f32);
    define!(f64);

    impl<T: FromEndian, const N: usize> FromEndian for [T; N] {
        fn from_be(&self) -> Self {
            let mut iter = self.iter();
            array::from_fn(|_| iter.next().unwrap().from_be())
        }
        fn from_le(&self) -> Self {
            let mut iter = self.iter();
            array::from_fn(|_| iter.next().unwrap().from_le())
        }
    }
}
