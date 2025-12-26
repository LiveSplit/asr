use bytemuck::{AnyBitPattern, CheckedBitPattern};
use core::{
    mem::{self, MaybeUninit},
    slice,
};

use crate::{Address, Address16, Address32, Address64, PointerSize};

use super::{sys, Error, MemoryRange};

pub use super::sys::ProcessId;

/// A process that the auto splitter is attached to.
#[repr(transparent)]
pub struct Process(pub(super) sys::Process);

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

    /// Attaches to a process based on its process id.
    #[inline]
    pub fn attach_by_pid(pid: ProcessId) -> Option<Self> {
        // SAFETY: We do proper error handling afterwards.
        let id = unsafe { sys::process_attach_by_pid(pid) };
        id.map(Self)
    }

    /// Lists processes based on their name. The processes are not in any
    /// specific order. Returns [`None`] if listing the processes failed. A buffer
    /// is provided that is filled with the process ids of the processes that
    /// were found. If the buffer is too small, the buffer is filled with as
    /// many process ids as possible. The length of the total amount of process
    /// ids that were found is also returned. This can be used to detect if the
    /// buffer was too small and can be used to either reallocate the buffer or
    /// to consider this an error condition.
    pub fn list_by_name_into<'buf>(
        name: &str,
        buf: &'buf mut [MaybeUninit<ProcessId>],
    ) -> Option<(&'buf mut [ProcessId], usize)> {
        // SAFETY: We provide a valid pointer and length to the name. The name
        // is guaranteed to be valid UTF-8. We also pass a pointer to the buffer
        // and its length. We also do proper error handling afterwards. The
        // buffer is guaranteed to be initialized afterwards, so we can safely
        // return a slice of it. The length is updated to be the amount of
        // process ids that were found. If this is smaller than the length of
        // the buffer, we slice the buffer to the length that was found.
        unsafe {
            let mut len = buf.len();

            let successful = sys::process_list_by_name(
                name.as_ptr(),
                name.len(),
                buf.as_mut_ptr().cast::<ProcessId>(),
                &mut len,
            );

            if !successful {
                return None;
            }

            let slice_len = len.min(buf.len());

            Some((
                slice::from_raw_parts_mut(buf.as_mut_ptr().cast::<ProcessId>(), slice_len),
                len,
            ))
        }
    }

    /// Lists processes based on their name. The processes are not in any
    /// specific order. Returns [`None`] if listing the processes failed. A
    /// vector is returned that is filled with the process ids of the processes
    /// that were found.
    #[cfg(feature = "alloc")]
    pub fn list_by_name(name: &str) -> Option<alloc::vec::Vec<ProcessId>> {
        // SAFETY: We provide a valid pointer and length to the name. The name
        // is guaranteed to be valid UTF-8. We call `process_list_by_name` with
        // a pointer to the allocated buffer and its capacity. The `len_cap`
        // will then be modified to be the amount of process ids that were
        // found. If this is less than or equal to the capacity, we know that
        // the buffer was large enough and we can return it. If it is larger, we
        // know that the buffer was too small and we need to call
        // `process_list_by_name` again with a larger buffer. We then repeat
        // this process until we have a buffer that is large enough.
        unsafe {
            let mut buf = alloc::vec::Vec::with_capacity(0);

            loop {
                // Passed in as the capacity of the buffer, later contains the
                // length that was actually needed.
                let mut len_cap = buf.capacity();

                let successful = sys::process_list_by_name(
                    name.as_ptr(),
                    name.len(),
                    buf.as_mut_ptr(),
                    &mut len_cap,
                );

                if !successful {
                    return None;
                }

                if len_cap <= buf.capacity() {
                    buf.set_len(len_cap);
                    return Some(buf);
                }

                buf.reserve(len_cap);
            }
        }
    }

    /// Checks whether the process is still open. If it is not open anymore, you
    /// should drop the process.
    #[inline]
    pub fn is_open(&self) -> bool {
        // SAFETY: The process handle is guaranteed to be valid.
        unsafe { sys::process_is_open(self.0) }
    }

    /// Gets the path of the executable in the file system. The path is a path
    /// that is accessible through the WASI file system, so a Windows path of
    /// `C:\foo\bar.exe` would be returned as `/mnt/c/foo/bar.exe`.
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn get_path(&self) -> Result<alloc::string::String, Error> {
        // SAFETY: Calling `process_get_path` with a null pointer and 0 length
        // will return the required length. We then allocate a buffer with the
        // required length and call it again with the buffer. We then convert
        // the buffer into a string, which is guaranteed to be valid UTF-8.
        unsafe {
            let mut len = 0;
            sys::process_get_path(self.0, core::ptr::null_mut(), &mut len);
            let mut buf = alloc::vec::Vec::with_capacity(len);
            let success = sys::process_get_path(self.0, buf.as_mut_ptr(), &mut len);
            if !success {
                return Err(Error {});
            }
            buf.set_len(len);
            Ok(alloc::string::String::from_utf8_unchecked(buf))
        }
    }

    /// Gets the name of the process.
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn get_name(&self) -> Result<alloc::string::String, Error> {
        let mut path = self.get_path()?;

        // remove everything before the / on path to avoid an allocation
        let (before, _) = path.rsplit_once('/').ok_or(Error {})?;
        let index = before.len() + 1;
        path.drain(..index);

        Ok(path)
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

    /// Gets the path of a module in the file system. The path is a path that is
    /// accessible through the WASI file system, so a Windows path of
    /// `C:\foo\bar.dll` would be returned as `/mnt/c/foo/bar.dll`.
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn get_module_path(&self, name: &str) -> Result<alloc::string::String, Error> {
        // SAFETY: Calling `process_get_module_path` with a null pointer and 0
        // length will return the required length. We then allocate a buffer
        // with the required length and call it again with the buffer. We then
        // convert the buffer into a string, which is guaranteed to be valid
        // UTF-8.
        unsafe {
            let mut len = 0;
            sys::process_get_module_path(
                self.0,
                name.as_ptr(),
                name.len(),
                core::ptr::null_mut(),
                &mut len,
            );
            let mut buf = alloc::vec::Vec::with_capacity(len);
            let success = sys::process_get_module_path(
                self.0,
                name.as_ptr(),
                name.len(),
                buf.as_mut_ptr(),
                &mut len,
            );
            if !success {
                return Err(Error {});
            }
            buf.set_len(len);
            Ok(alloc::string::String::from_utf8_unchecked(buf))
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

    /// Get the address and size of the main module in the process.
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn get_main_module_range(&self) -> Result<(Address, u64), Error> {
        let main_module_name = self.get_name()?;
        self.get_module_range(&main_module_name)
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

    /// Reads a range of bytes from the process at the address given into the
    /// buffer provided. This is a convenience method for reading into a slice
    /// of a specific type. The buffer does not need to be initialized. After
    /// the slice successfully got filled, the initialized slice is returned.
    #[inline]
    pub fn read_into_uninit_slice<T: CheckedBitPattern>(
        &self,
        address: impl Into<Address>,
        slice: &mut [MaybeUninit<T>],
    ) -> Result<&mut [T], Error> {
        // SAFETY: The process handle is guaranteed to be valid. We provide a
        // valid pointer and length to the buffer. We also do proper error
        // handling afterwards. The buffer is guaranteed to be initialized
        // afterwards, we just need to check if the values are valid bit
        // patterns. We can then safely return a slice of it.
        unsafe {
            let byte_len = mem::size_of_val(slice);
            self.read_into_uninit_buf(
                address,
                slice::from_raw_parts_mut(slice.as_mut_ptr().cast(), byte_len),
            )?;
            for element in &*slice {
                if !T::is_valid_bit_pattern(&*element.as_ptr().cast::<T::Bits>()) {
                    return Err(Error {});
                }
            }
            let len = slice.len();
            Ok(slice::from_raw_parts_mut(slice.as_mut_ptr().cast(), len))
        }
    }

    /// Reads an array from the process at the address with the length given
    /// into the `Vec` provided. The `Vec` is not cleared, all elements are
    /// appended to the end of the `Vec`. You may want to manually clear it
    /// beforehand.
    #[cfg(feature = "alloc")]
    pub fn append_to_vec<T: CheckedBitPattern>(
        &self,
        address: impl Into<Address>,
        vec: &mut alloc::vec::Vec<T>,
        additional_elements: usize,
    ) -> Result<(), Error> {
        let new_len = vec.len().saturating_add(additional_elements);
        if new_len > isize::MAX as usize {
            return Err(Error {});
        }

        vec.reserve(additional_elements);

        // SAFETY: The length is only set after the elements are successfully
        // read into the vector.
        unsafe {
            self.read_into_uninit_slice(
                address,
                &mut vec.spare_capacity_mut()[..additional_elements],
            )?;
            vec.set_len(new_len);
        }

        Ok(())
    }

    /// Reads an array from the process at the address with the length given into
    /// a new `Vec`. This is a heap allocation. It's recommended to avoid this
    /// method if possible and either use [`read`](Self::read) with a fixed size
    /// array or [`read_into_slice`](Self::read_into_slice) if possible. If
    /// neither of these are possible it is recommend to at least reuse the
    /// `Vec` with [`append_to_vec`](Self::append_to_vec) if that's possible.
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn read_vec<T: CheckedBitPattern>(
        &self,
        address: impl Into<Address>,
        len: usize,
    ) -> Result<alloc::vec::Vec<T>, Error> {
        let mut buf = alloc::vec::Vec::new();
        self.append_to_vec(address, &mut buf, len)?;
        Ok(buf)
    }

    /// Reads a pointer address from the process at the address given.
    pub fn read_pointer(
        &self,
        address: impl Into<Address>,
        pointer_size: PointerSize,
    ) -> Result<Address, Error> {
        Ok(match pointer_size {
            PointerSize::Bit16 => self.read::<Address16>(address)?.into(),
            PointerSize::Bit32 => self.read::<Address32>(address)?.into(),
            PointerSize::Bit64 => self.read::<Address64>(address)?.into(),
        })
    }

    /// Follows a path of pointers from the address given and reads a value of
    /// the type specified from the process at the end of the pointer path.
    pub fn read_pointer_path<T: CheckedBitPattern>(
        &self,
        address: impl Into<Address>,
        pointer_size: PointerSize,
        path: &[u64],
    ) -> Result<T, Error> {
        let mut address = address.into();
        let (&last, path) = path.split_last().ok_or(Error {})?;
        for &offset in path {
            address = self.read_pointer(address + offset, pointer_size)?;
        }
        self.read(address + last)
    }
}
