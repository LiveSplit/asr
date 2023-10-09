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
            base_address: Address::NULL,
            path: ArrayVec::new(),
            deref_type: DerefType::Bit64,
        }
    }
}

impl<const CAP: usize> DeepPointer<CAP> {
    /// Creates a new DeepPointer.
    #[inline]
    pub fn new(base_address: Address, deref_type: DerefType, path: &[u64]) -> Self {
        assert!(CAP != 0 && CAP >= path.len());

        let mut deref_path = ArrayVec::new();
        for &val in path {
            deref_path.push(val);
        }

        Self {
            base_address,
            path: deref_path,
            deref_type,
        }
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
#[derive(Copy, Clone)]
pub enum DerefType {
    /// 4-byte pointer size, used in 32bit processes
    Bit32,
    /// 8-byte pointer size, used in 64bit processes
    Bit64,
}
