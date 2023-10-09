//! Support for storing pointer paths for easy dereferencing inside the autosplitter logic.

use arrayvec::ArrayVec;
use bytemuck::CheckedBitPattern;

use crate::{Address, Address32, Address64, Error, Process};

/// An abstraction of a pointer path, usable for easy dereferencing inside an autosplitter logic.
///
/// The maximum depth of the pointer path is given by the generic parameter `CAP`.
/// Of note, `CAP` must be higher or equal to the number of offsets provided in `path`,
/// otherwise calling `new()` on this struct will trigger a ***Panic***.
#[derive(Clone)]
pub struct DeepPointer<const CAP: usize> {
    base_address: Address,
    path: ArrayVec<u64, CAP>,
    deref_type: DerefType,
}

impl<const CAP: usize> Default for DeepPointer<CAP> {
    /// Creates a new empty DeepPointer.
    #[inline]
    fn default() -> Self {
        Self {
            base_address: Address::default(),
            path: ArrayVec::default(),
            deref_type: DerefType::default(),
        }
    }
}

impl<const CAP: usize> DeepPointer<CAP> {
    /// Creates a new DeepPointer and specify the pointer size dereferencing
    #[inline]
    pub fn new(base_address: Address, deref_type: DerefType, path: &[u64]) -> Self {
        assert!(CAP != 0 && CAP >= path.len());
        Self {
            base_address,
            path: path.iter().cloned().collect(),
            deref_type,
        }
    }

    /// Creates a new DeepPointer with 32bit pointer size dereferencing
    pub fn new_32bit(base_address: Address, path: &[u64]) -> Self {
        Self::new(base_address, DerefType::Bit32, path)
    }

    /// Creates a new DeepPointer with 64bit pointer size dereferencing
    pub fn new_64bit(base_address: Address, path: &[u64]) -> Self {
        Self::new(base_address, DerefType::Bit64, path)
    }

    /// Dereferences the pointer path, returning the memory address of the value of interest
    pub fn deref_offsets(&self, process: &Process) -> Result<Address, Error> {
        let mut address = self.base_address;
        let (&last, path) = self.path.split_last().ok_or(Error {})?;
        for &offset in path {
            address = match self.deref_type {
                DerefType::Bit32 => process.read::<Address32>(address + offset)?.into(),
                DerefType::Bit64 => process.read::<Address64>(address + offset)?.into(),
            };
        }
        Ok(address + last)
    }

    /// Dereferences the pointer path, returning the value stored at the final memory address
    pub fn deref<T: CheckedBitPattern>(&self, process: &Process) -> Result<T, Error> {
        process.read(self.deref_offsets(process)?)
    }
}

/// Describes the pointer size that should be used while deferecencing a pointer path
#[derive(Copy, Clone, Default)]
pub enum DerefType {
    /// 4-byte pointer size, used in 32bit processes
    Bit32,
    /// 8-byte pointer size, used in 64bit processes
    #[default]
    Bit64,
}
