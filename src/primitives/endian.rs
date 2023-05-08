use core::array;

use crate::{Address16, Address32, Address64};

/// The endianness of a value.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Endian {
    /// Big endian.
    Big,
    /// Little endian.
    Little,
}

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
                Self::from_be_bytes(bytemuck::cast(*self))
            }
            fn from_le(&self) -> Self {
                Self::from_le_bytes(bytemuck::cast(*self))
            }
        }
    };
}

macro_rules! define_addr {
    ($name:ident) => {
        impl FromEndian for $name {
            fn from_be(&self) -> Self {
                Self::new(self.value().from_be())
            }
            fn from_le(&self) -> Self {
                Self::new(self.value().from_le())
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

define_addr!(Address16);
define_addr!(Address32);
define_addr!(Address64);

impl FromEndian for bool {
    fn from_be(&self) -> Self {
        *self
    }
    fn from_le(&self) -> Self {
        *self
    }
}

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
