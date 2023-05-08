use crate::Address;

use super::{sys, Error, Process};

#[cfg(feature = "flags")]
bitflags::bitflags! {
    /// Describes various flags of a memory range.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct MemoryRangeFlags: u64 {
        /// The memory range is readable.
        const READ = 1 << 1;
        /// The memory range is writable.
        const WRITE = 1 << 2;
        /// The memory range is executable.
        const EXECUTE = 1 << 3;
        /// The memory range has a file path.
        const PATH = 1 << 4;
    }
}

/// A memory range of a process. All information is queried lazily.
#[derive(Copy, Clone)]
pub struct MemoryRange<'a> {
    pub(crate) process: &'a Process,
    pub(crate) index: u64,
}

impl MemoryRange<'_> {
    /// Queries the starting address of the memory range.
    #[inline]
    pub fn address(&self) -> Result<Address, Error> {
        // SAFETY: The process is guaranteed to be valid because we borrow an
        // owned Process object.
        unsafe {
            let address = sys::process_get_memory_range_address(self.process.0, self.index);
            if let Some(address) = address {
                Ok(Address::new(address.0.get()))
            } else {
                Err(Error {})
            }
        }
    }

    /// Queries the size of the memory range.
    #[inline]
    pub fn size(&self) -> Result<u64, Error> {
        // SAFETY: The process is guaranteed to be valid because we borrow an
        // owned Process object.
        unsafe {
            let size = sys::process_get_memory_range_size(self.process.0, self.index);
            if let Some(size) = size {
                Ok(size.get())
            } else {
                Err(Error {})
            }
        }
    }

    /// Queries the flags of the memory range.
    #[cfg(feature = "flags")]
    #[inline]
    pub fn flags(&self) -> Result<MemoryRangeFlags, Error> {
        // SAFETY: The process is guaranteed to be valid because we borrow an
        // owned Process object.
        unsafe {
            let flags = sys::process_get_memory_range_flags(self.process.0, self.index);
            if let Some(flags) = flags {
                Ok(MemoryRangeFlags::from_bits_truncate(flags.get()))
            } else {
                Err(Error {})
            }
        }
    }
}
