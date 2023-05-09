use bytemuck::{AnyBitPattern, CheckedBitPattern};
use core::{
    mem::{self, MaybeUninit},
    slice,
};

use crate::{Address, Address32, Address64};

use super::{
    sys::{self, ProcessId},
    Error, MemoryRange,
};

/// A process that the auto splitter is attached to.
#[derive(Debug)]
#[repr(transparent)]
pub struct Process(pub(super) ProcessId);

impl Drop for Process {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: The handle is guaranteed to be valid because the only way to
        // construct this type is through `Process::attach`, which guarantees a
        // valid handle. Also it is guaranteed to still be attached, as the only
        // place that detaches it, is in `Drop`.
        unsafe { sys::process_detach(self.0) }
    }
}

impl Process {
    /// Attaches to a process based on its name.
    #[inline]
    pub fn attach(name: &str) -> Option<Self> {
        // SAFETY: We provide a valid pointer and length to the name. The name
        // is guaranteed to be valid UTF-8. We also do proper error handling
        // afterwards.
        let id = unsafe { sys::process_attach(name.as_ptr(), name.len()) };
        id.map(Self)
    }

    /// Checks whether the process is still open. If it is not open anymore, you
    /// should drop the process.
    #[inline]
    pub fn is_open(&self) -> bool {
        // SAFETY: The process handle is guaranteed to be valid.
        unsafe { sys::process_is_open(self.0) }
    }

    /// Gets the address of a module in the process.
    #[inline]
    pub fn get_module_address(&self, name: &str) -> Result<Address, Error> {
        // SAFETY: The process handle is guaranteed to be valid. We provide a
        // valid pointer and length to the name. The name is guaranteed to be
        // valid UTF-8. We also do proper error handling afterwards.
        unsafe {
            let address = sys::process_get_module_address(self.0, name.as_ptr(), name.len());
            if let Some(address) = address {
                Ok(Address::new(address.0.get()))
            } else {
                Err(Error {})
            }
        }
    }

    /// Gets the size of a module in the process.
    #[inline]
    pub fn get_module_size(&self, name: &str) -> Result<u64, Error> {
        // SAFETY: The process handle is guaranteed to be valid. We provide a
        // valid pointer and length to the name. The name is guaranteed to be
        // valid UTF-8. We also do proper error handling afterwards.
        unsafe {
            let size = sys::process_get_module_size(self.0, name.as_ptr(), name.len());
            if let Some(size) = size {
                Ok(size.get())
            } else {
                Err(Error {})
            }
        }
    }

    /// Gets the address and size of a module in the process.
    #[inline]
    pub fn get_module_range(&self, name: &str) -> Result<(Address, u64), Error> {
        Ok((self.get_module_address(name)?, self.get_module_size(name)?))
    }

    /// Iterates over all committed (not reserved, not free) memory ranges of the process.
    #[inline]
    pub fn memory_ranges(&self) -> impl DoubleEndedIterator<Item = MemoryRange<'_>> {
        // SAFETY: The process handle is guaranteed to be valid. We handle the
        // error by returning an empty iterator.
        let count = unsafe { sys::process_get_memory_range_count(self.0).map_or(0, |c| c.get()) };
        (0..count).map(|index| MemoryRange {
            process: self,
            index,
        })
    }

    /// Reads a value of the type specified from the process at the address
    /// given.
    #[inline]
    pub fn read<T: CheckedBitPattern>(&self, address: impl Into<Address>) -> Result<T, Error> {
        // SAFETY: The process handle is guaranteed to be valid. We provide a
        // valid pointer and length to the uninitialized value. We also do
        // proper error handling after reading into it. At that point we know
        // that the bytes of the value are fully initialized. We then check if
        // the value is a valid bit pattern for the type. We can then assume
        // that the value is valid and return it.
        unsafe {
            let mut value = MaybeUninit::<T>::uninit();
            self.read_into_uninit_buf(
                address,
                slice::from_raw_parts_mut(value.as_mut_ptr().cast(), mem::size_of::<T>()),
            )?;
            if !T::is_valid_bit_pattern(&*value.as_ptr().cast::<T::Bits>()) {
                return Err(Error {});
            }
            Ok(value.assume_init())
        }
    }

    /// Reads a range of bytes from the process at the address given into the
    /// buffer provided.
    #[inline]
    pub fn read_into_buf(&self, address: impl Into<Address>, buf: &mut [u8]) -> Result<(), Error> {
        // SAFETY: The process handle is guaranteed to be valid. We provide a
        // valid pointer and length to the buffer. We also do proper error
        // handling afterwards.
        unsafe {
            let buf_len = buf.len();
            if sys::process_read(self.0, address.into(), buf.as_mut_ptr(), buf_len) {
                Ok(())
            } else {
                Err(Error {})
            }
        }
    }

    /// Reads a range of bytes from the process at the address given into the
    /// buffer provided. The buffer does not need to be initialized. After the
    /// buffer successfully got filled, the initialized buffer is returned.
    #[inline]
    pub fn read_into_uninit_buf<'buf>(
        &self,
        address: impl Into<Address>,
        buf: &'buf mut [MaybeUninit<u8>],
    ) -> Result<&'buf mut [u8], Error> {
        // SAFETY: The process handle is guaranteed to be valid. We provide a
        // valid pointer and length to the buffer. We also do proper error
        // handling afterwards. The buffer is guaranteed to be initialized
        // afterwards, so we can safely return an u8 slice of it.
        unsafe {
            let buf_len = buf.len();
            if sys::process_read(self.0, address.into(), buf.as_mut_ptr().cast(), buf_len) {
                Ok(slice::from_raw_parts_mut(buf.as_mut_ptr().cast(), buf_len))
            } else {
                Err(Error {})
            }
        }
    }

    /// Reads a range of bytes from the process at the address given into the
    /// buffer provided. This is a convenience method for reading into a slice
    /// of a specific type.
    #[inline]
    pub fn read_into_slice<T: AnyBitPattern>(
        &self,
        address: impl Into<Address>,
        slice: &mut [T],
    ) -> Result<(), Error> {
        // SAFETY: The process handle is guaranteed to be valid. We provide a
        // valid pointer and length to the buffer. We also restrict the type to
        // `AnyBitPattern` as opposed to `CheckedBitPattern` because we can't
        // undo the changes to the slice if the validity check fails. We do
        // proper error handling afterwards.
        unsafe {
            let len = mem::size_of_val(slice);
            self.read_into_uninit_buf(
                address,
                slice::from_raw_parts_mut(slice.as_mut_ptr().cast(), len),
            )?;
            Ok(())
        }
    }

    /// Follows a path of pointers from the address given and reads a value of
    /// the type specified from the process at the end of the pointer path. This
    /// method is specifically for dealing with processes that use 64-bit
    /// pointers.
    pub fn read_pointer_path64<T: CheckedBitPattern>(
        &self,
        address: impl Into<Address>,
        path: &[u64],
    ) -> Result<T, Error> {
        let mut address = address.into();
        let (&last, path) = path.split_last().ok_or(Error {})?;
        for &offset in path {
            address = self.read::<Address64>(address + offset)?.into();
        }
        self.read(address + last)
    }

    /// Follows a path of pointers from the address given and reads a value of
    /// the type specified from the process at the end of the pointer path. This
    /// method is specifically for dealing with processes that use 32-bit
    /// pointers.
    pub fn read_pointer_path32<T: CheckedBitPattern>(
        &self,
        address: impl Into<Address>,
        path: &[u32],
    ) -> Result<T, Error> {
        let mut address = address.into();
        let (&last, path) = path.split_last().ok_or(Error {})?;
        for &offset in path {
            address = self.read::<Address32>(address + offset)?.into();
        }
        self.read(address + last)
    }
}
