//! Support for storing pointer paths for easy dereferencing inside the autosplitter logic.

use core::array;

use bytemuck::CheckedBitPattern;

use crate::{Address, Error, PointerSize, Process};

/// An abstraction of a pointer path, usable for easy dereferencing inside an autosplitter logic.
///
/// The maximum depth of the pointer path is given by the generic parameter `CAP`.
///
/// `CAP` should be higher or equal to the number of offsets provided in `path`.
/// If a higher number of offsets is provided, the pointer path will be truncated
/// according to the value of `CAP`.
#[derive(Copy, Clone)]
pub struct DeepPointer<const CAP: usize> {
    base_address: Address,
    path: [u64; CAP],
    depth: usize,
    pointer_size: PointerSize,
}

impl<const CAP: usize> Default for DeepPointer<CAP> {
    /// Creates a new empty DeepPointer.
    #[inline]
    fn default() -> Self {
        Self {
            base_address: Address::default(),
            path: [u64::default(); CAP],
            depth: usize::default(),
            pointer_size: PointerSize::Bit64,
        }
    }
}

impl<const CAP: usize> DeepPointer<CAP> {
    /// Creates a new DeepPointer and specify the pointer size dereferencing
    #[inline]
    pub fn new(base_address: impl Into<Address>, pointer_size: PointerSize, path: &[u64]) -> Self {
        let this_path = {
            let mut iter = path.iter();
            array::from_fn(|_| iter.next().copied().unwrap_or_default())
        };

        Self {
            base_address: base_address.into(),
            path: this_path,
            depth: path.len().min(CAP),
            pointer_size,
        }
    }

    /// Creates a new DeepPointer with 32bit pointer size dereferencing
    pub fn new_32bit(base_address: impl Into<Address>, path: &[u64]) -> Self {
        Self::new(base_address, PointerSize::Bit32, path)
    }

    /// Creates a new DeepPointer with 64bit pointer size dereferencing
    pub fn new_64bit(base_address: impl Into<Address>, path: &[u64]) -> Self {
        Self::new(base_address, PointerSize::Bit64, path)
    }

    /// Dereferences the pointer path, returning the memory address of the value of interest
    pub fn deref_offsets(&self, process: &Process) -> Result<Address, Error> {
        let mut address = self.base_address;
        let (&last, path) = self.path[..self.depth].split_last().ok_or(Error {})?;
        for &offset in path {
            address = process.read_pointer(address + offset, self.pointer_size)?;
        }
        Ok(address + last)
    }

    /// Dereferences the pointer path, returning the value stored at the final memory address
    pub fn deref<T: CheckedBitPattern>(&self, process: &Process) -> Result<T, Error> {
        process.read_pointer_path(
            self.base_address,
            self.pointer_size,
            &self.path[..self.depth],
        )
    }
}
