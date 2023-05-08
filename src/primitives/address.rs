use core::{fmt, ops::Add};

use bytemuck::{Pod, Zeroable};

macro_rules! define_addr {
    (#[$name_in_doc:meta] $name:ident => $inner_u:ident => $inner_i:ident) => {
        #[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        /// A
        #[$name_in_doc]
        pub struct $name($inner_u);

        impl $name {
            /// The null pointer pointing to address 0.
            pub const NULL: Self = Self(0);

            /// Creates a new address from the given value.
            #[inline]
            pub const fn new(value: $inner_u) -> Self {
                Self(value)
            }

            /// Returns the underlying address as an integer.
            #[inline]
            pub const fn value(self) -> $inner_u {
                self.0
            }

            /// Checks whether the address is null.
            #[inline]
            pub const fn is_null(self) -> bool {
                self.0 == 0
            }

            /// Offsets the address by the given number of bytes.
            #[inline]
            pub const fn add(self, bytes: $inner_u) -> Self {
                Self(self.0.wrapping_add(bytes))
            }

            /// Offsets the address by the given number of bytes.
            #[inline]
            pub const fn add_signed(self, bytes: $inner_i) -> Self {
                Self(self.0.wrapping_add_signed(bytes))
            }
        }

        impl fmt::Debug for $name {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Pointer::fmt(self, f)
            }
        }

        impl fmt::Display for $name {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Pointer::fmt(self, f)
            }
        }

        impl fmt::Pointer for $name {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::LowerHex::fmt(&self.0, f)
            }
        }
    };
}

macro_rules! define_pod_addr {
    (#[$name_in_doc:meta] $name:ident => $inner_u:ident => $inner_i:ident) => {
        define_addr!(#[$name_in_doc] $name => $inner_u => $inner_i);

        impl From<$name> for Address {
            #[inline]
            fn from(addr: $name) -> Self {
                Self(addr.0 as _)
            }
        }

        impl From<$inner_u> for Address {
            #[inline]
            fn from(addr: $inner_u) -> Self {
                Self(addr as _)
            }
        }

        impl Add<$inner_u> for Address {
            type Output = Self;

            #[inline]
            fn add(self, bytes: $inner_u) -> Self {
                Self(self.0.wrapping_add(bytes as _))
            }
        }

        impl Add<$inner_i> for Address {
            type Output = Self;

            #[inline]
            fn add(self, bytes: $inner_i) -> Self {
                Self(self.0.wrapping_add_signed(bytes as _))
            }
        }

        impl From<$inner_u> for $name {
            #[inline]
            fn from(addr: $inner_u) -> Self {
                Self(addr as _)
            }
        }

        impl Add<$inner_u> for $name {
            type Output = Self;

            #[inline]
            fn add(self, bytes: $inner_u) -> Self {
                Self(self.0.wrapping_add(bytes as _))
            }
        }

        impl Add<$inner_i> for $name {
            type Output = Self;

            #[inline]
            fn add(self, bytes: $inner_i) -> Self {
                Self(self.0.wrapping_add_signed(bytes as _))
            }
        }

        // SAFETY: The type is transparent over an integer, which is `Pod`.
        unsafe impl Pod for $name {}
        // SAFETY: The type is transparent over an integer, which is `Zeroable`.
        unsafe impl Zeroable for $name {}
    };
}

define_pod_addr!(#[doc = "16-bit address that can be read from a process's memory."] Address16 => u16 => i16);
define_pod_addr!(#[doc = "32-bit address that can be read from a process's memory."] Address32 => u32 => i32);
define_pod_addr!(#[doc = "64-bit address that can be read from a process's memory."] Address64 => u64 => i64);
define_addr!(#[doc = "general purpose address."] Address => u64 => i64);
