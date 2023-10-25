mod address;
mod endian;

pub use self::{address::*, endian::*};

/// Pointer size represents the width (in bytes) of memory addresses used
/// in a certain process.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[repr(u8)]
pub enum PointerSize {
    /// A 16-bit (2 byte wide) pointer size
    Bit16 = 0x2,
    /// A 32-bit (4 byte wide) pointer size
    Bit32 = 0x4,
    /// A 64-bit (8 byte wide) pointer size
    Bit64 = 0x8,
}
