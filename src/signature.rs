//! Support for finding patterns in a process's memory.

use crate::{future::retry, Address, Process};
use core::{
    iter,
    mem::{self, MaybeUninit},
    slice,
};

type Offset = u8;

/// A signature that can be used to find a pattern in a process's memory.
/// It is recommended to store this in a `static` or `const` variable to ensure
/// that the signature is parsed at compile time, which allows optimizations.
/// Also, compiling with the `simd128` feature is recommended for SIMD support.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Signature<const N: usize> {
    /// A simple signature that does not contain any wildcards.
    Simple([u8; N]),
    /// A complex signature that contains wildcards.
    Complex {
        /// The signature itself.
        needle: [u8; N],
        /// The mask that indicates which bytes are wildcards.
        mask: [u8; N],
        /// A lookup table of offsets to jump forward by when certain bytes are encountered.
        skip_offsets: [Offset; 256],
    },
}

/// A helper struct to parse a hexadecimal signature string into bytes.
struct Parser<'a> {
    bytes: &'a [u8],
}

impl Parser<'_> {
    #[inline]
    const fn next(mut self) -> (Option<u8>, Self) {
        while let [b, rem @ ..] = self.bytes {
            self.bytes = rem;
            let b: u8 = *b;
            return (
                Some(match b {
                    b'0'..=b'9' => b - b'0',       // Convert '0'-'9' to their numeric value
                    b'a'..=b'f' => b - b'a' + 0xA, // Convert 'a'-'f' to their numeric value
                    b'A'..=b'F' => b - b'A' + 0xA, // Convert 'A'-'F' to their numeric value
                    b'?' => 0x10,                  // Treat wildcard ('?') as a special byte
                    b' ' | b'\r' | b'\n' | b'\t' => continue, // Skip whitespace
                    _ => panic!("Invalid byte"),   // Invalid characters cause a panic
                }),
                self,
            );
        }
        (None, self)
    }
}

/// Checks if a slice of bytes contains the specified `search_byte`.
#[inline]
const fn contains(mut bytes: &[u8], search_byte: u8) -> bool {
    while let [b, rem @ ..] = bytes {
        bytes = rem;
        if *b == search_byte {
            return true;
        }
    }
    false
}

impl<const N: usize> Signature<N> {
    /// Creates a new signature from a string. The string must be a hexadecimal
    /// string with `?` as wildcard. It is recommended to store this in a
    /// `static` or `const` variable to ensure that the signature is parted
    /// at compile time, which allows optimizations.
    ///
    /// # Panics
    ///
    /// This function panics if the signature is invalid or if its length
    /// exceeds 255 bytes.
    ///
    /// # Example
    ///
    /// ```
    /// # use asr::signature::Signature;
    /// static SIG: Signature<8> = Signature::new("3A 45 FF ?? ?? B? 00 12");
    /// ```
    pub const fn new(signature: &str) -> Self {
        // We only support u8 offsets atm and thus signatures can't be 256 bytes
        // or longer.
        assert!(N > 0 && N < 256);

        let mut parser = Parser {
            bytes: signature.as_bytes(),
        };

        // Check if the signature contains wildcards
        if contains(signature.as_bytes(), b'?') {
            let mut needle = [0; N];
            let mut mask = [0; N];
            let mut i = 0;

            loop {
                let (a, next) = parser.next();
                parser = next;
                let (b, next) = parser.next();
                parser = next;
                let (Some(a), Some(b)) = (a, b) else { break };
                let sig_byte = (a << 4) | (b & 0x0F);
                let mask_byte = ((a != 0x10) as u8 * 0xF0) | ((b != 0x10) as u8 * 0x0F);
                needle[i] = sig_byte & mask_byte;
                mask[i] = mask_byte;
                i += 1;
            }
            assert!(i == N); // Ensure we parsed the correct number of bytes

            // Initialize skip_offsets with all zeros
            let mut skip_offsets = [0; 256];

            let mut unknown = 0;
            let end = N - 1;
            let mut i = 0;

            while i < end {
                let byte = needle[i];
                let mask = mask[i];
                if mask == 0xFF {
                    skip_offsets[byte as usize] = (end - i) as Offset;
                } else {
                    unknown = (end - i) as Offset;
                }
                i += 1;
            }

            if unknown == 0 {
                unknown = N as Offset;
            }

            // Set the skip offsets for any byte that wasn't explicitly set
            i = 0;
            while i < skip_offsets.len() {
                if unknown < skip_offsets[i] || skip_offsets[i] == 0 {
                    skip_offsets[i] = unknown;
                }
                i += 1;
            }

            Self::Complex {
                needle,
                mask,
                skip_offsets,
            }
        } else {
            // If the provided string has no wildcards, treat as a Simple signature
            let mut needle = [0; N];
            let mut i = 0;

            loop {
                let (a, next) = parser.next();
                parser = next;
                let (b, next) = parser.next();
                parser = next;
                let (Some(a), Some(b)) = (a, b) else { break };
                let sig_byte = (a << 4) | b;
                needle[i] = sig_byte;
                i += 1;
            }
            assert!(i == N);

            Self::Simple(needle)
        }
    }

    /// Performs a signature scan over a provided slice.
    /// Returns an iterator over the positions where the signature matches.
    fn scan_internal<'a>(&'a self, haystack: &'a [u8]) -> impl Iterator<Item = usize> + 'a {
        let mut cursor = 0;
        let end = haystack.len().saturating_sub(N.saturating_sub(1));

        iter::from_fn(move || 'outer: loop {
            if cursor >= end {
                return None;
            }

            match self {
                Signature::Simple(needle) => {
                    match memchr::memmem::find(&haystack[cursor..], needle) {
                        Some(offset) => {
                            let current_cursor = cursor;
                            cursor += offset + 1;
                            return Some(offset + current_cursor);
                        }
                        None => return None,
                    };
                }
                Signature::Complex {
                    needle,
                    mask,
                    skip_offsets,
                } => {
                    let mut i = 0;

                    unsafe {
                        let (scan, mut needle, mut mask) = (
                            haystack.as_ptr().add(cursor),
                            needle.as_ptr(),
                            mask.as_ptr(),
                        );

                        #[cfg(target_feature = "simd128")]
                        while i + 16 <= N {
                            use core::arch::wasm32::{u8x16_ne, v128, v128_and, v128_any_true};

                            if v128_any_true(u8x16_ne(
                                v128_and(
                                    scan.add(i).cast::<v128>().read_unaligned(),
                                    mask.cast::<v128>().read_unaligned(),
                                ),
                                needle.cast::<v128>().read_unaligned(),
                            )) {
                                cursor +=
                                    skip_offsets[*scan.add(N.saturating_sub(1)) as usize] as usize;
                                continue 'outer;
                            } else {
                                mask = mask.add(16);
                                needle = needle.add(16);
                                i += 16;
                            }
                        }

                        while i + 8 <= N {
                            if scan.add(i).cast::<u64>().read_unaligned()
                                & mask.cast::<u64>().read_unaligned()
                                != needle.cast::<u64>().read_unaligned()
                            {
                                cursor +=
                                    skip_offsets[*scan.add(N.saturating_sub(1)) as usize] as usize;
                                continue 'outer;
                            } else {
                                mask = mask.add(8);
                                needle = needle.add(8);
                                i += 8;
                            }
                        }

                        while i + 4 <= N {
                            if scan.add(i).cast::<u32>().read_unaligned()
                                & mask.cast::<u32>().read_unaligned()
                                != needle.cast::<u32>().read_unaligned()
                            {
                                cursor +=
                                    skip_offsets[*scan.add(N.saturating_sub(1)) as usize] as usize;
                                continue 'outer;
                            } else {
                                mask = mask.add(4);
                                needle = needle.add(4);
                                i += 4;
                            }
                        }

                        while i + 2 <= N {
                            if scan.add(i).cast::<u16>().read_unaligned()
                                & mask.cast::<u16>().read_unaligned()
                                != needle.cast::<u16>().read_unaligned()
                            {
                                cursor +=
                                    skip_offsets[*scan.add(N.saturating_sub(1)) as usize] as usize;
                                continue 'outer;
                            } else {
                                mask = mask.add(2);
                                needle = needle.add(2);
                                i += 2;
                            }
                        }

                        while i < N {
                            if *scan.add(i) & *mask != *needle {
                                cursor +=
                                    skip_offsets[*scan.add(N.saturating_sub(1)) as usize] as usize;
                                continue 'outer;
                            } else {
                                mask = mask.add(1);
                                needle = needle.add(1);
                                i += 1;
                            }
                        }

                        let current_cursor = cursor;
                        cursor = cursor + 1;
                        return Some(current_cursor);
                    }
                }
            }
        })
        .fuse()
    }

    /// Scans a process's memory in the given range for the first occurrence of the signature.
    ///
    /// # Arguments
    ///
    /// * `process` - A reference to the `Process` in which the scan occurs.
    /// * `range` - A tuple containing:
    ///     - The starting address of the memory range
    ///     - The length of the memory range to scan
    ///
    /// Returns `Some(Address)` of the first match if found, otherwise `None`.
    pub fn scan_process_range<'a>(
        &'a self,
        process: &'a Process,
        range: (impl Into<Address>, u64),
    ) -> Option<Address> {
        self.scan_iter(process, range).next()
    }

    /// Returns an iterator over all occurrences of the signature in the process's memory range.
    ///
    /// # Arguments
    ///
    /// * `process` - A reference to the `Process` in which the scan occurs.
    /// * `range` - A tuple containing:
    ///     - The starting address of the memory range
    ///     - The length of the memory range to scan
    ///
    /// Returns an iterator that yields each matching address.
    pub fn scan_iter<'a>(
        &'a self,
        process: &'a Process,
        range: (impl Into<Address>, u64),
    ) -> impl Iterator<Item = Address> + 'a {
        const MEM_SIZE: usize = 0x1000;

        let mut addr: Address = Into::into(range.0);
        let overall_end = addr.value() + range.1;

        // The sigscan essentially works by reading one memory page (0x1000 bytes)
        // at a time and looking for the signature in each page. We will create a buffer
        // sligthly larger than 0x1000 bytes in order to accomodate the size of
        // the memory page + the signature - 1. The very first bytes of the
        // buffer will be used as the tail of the previous memory page.
        // This allows to scan across the memory page boundaries.

        // The buffer struct is essentially an array with the size of MEM_SIZE + N - 1
        #[repr(packed)]
        struct Buffer<const N: usize> {
            _head: [u8; N],
            _buffer: [u8; MEM_SIZE.saturating_sub(1)],
        }
        let mut buffer = MaybeUninit::<Buffer<N>>::uninit();

        // The tail of the previous memory page, if read correctly, is stored here
        let mut last_page_success = false;

        // SAFETY: The buffer is not initialized, so we purposefully make a slice of MaybeUninit<u8>, for which initialization is not required.
        // In order to ensure memory safety, we need to ensure the data is written before transmuting.
        let buffer = unsafe {
            slice::from_raw_parts_mut(
                buffer.as_mut_ptr() as *mut MaybeUninit<u8>,
                size_of::<Buffer<N>>(),
            )
        };

        iter::from_fn(move || {
            if addr.value() >= overall_end {
                return None;
            }

            // We round up to the 4 KiB address boundary as that's a single
            // page, which is safe to read either fully or not at all. We do
            // this to reduce the number of syscalls as much as possible, as the
            // syscall overhead is quite high.
            let end = ((addr.value() & !((4 << 10) - 1)) + (4 << 10)).min(overall_end);
            let len = end.saturating_sub(addr.value()) as usize;

            // If we have read the previous memory page successfully, then we can copy the last
            // elements to the start of the buffer.
            if last_page_success {
                // This of course assumes that N is always lower than MEM_SIZE.
                // For the current implementation, this is always true.
                let (start, end) = buffer.split_at_mut(N.saturating_sub(1));
                start.copy_from_slice(&end[MEM_SIZE..]);
            }

            let current_page_success = process
                .read_into_uninit_buf(addr, &mut buffer[N.saturating_sub(1)..][..len])
                .is_ok();

            // We define the final slice on which to perform the memory scan into. If we failed to read the memory page,
            // this returns an empty slice so the subsequent iterator will result into an empty iterator.
            // If we managed to read the current memory page, instead, we check if we have successfully read the data
            // from the previous memory page.
            let scan_buf = unsafe {
                let ptr = if current_page_success {
                    if last_page_success {
                        &buffer[..len + N.saturating_sub(1)]
                    } else {
                        &buffer[N.saturating_sub(1)..][..len]
                    }
                } else {
                    &[]
                };

                mem::transmute::<&[MaybeUninit<u8>], &[u8]>(ptr)
            };

            let cur_addr = addr;
            let cur_suc = last_page_success;

            addr = Address::new(end);
            last_page_success = current_page_success;

            Some(self.scan_internal(&scan_buf).map(move |pos| {
                let mut address = cur_addr.add(pos as u64);

                if cur_suc {
                    address = address.add_signed(-((N as i64).saturating_sub(1)))
                }

                address
            }))
        })
        .flatten()
    }

    /// Asynchronously awaits scanning a process for the signature until a match
    /// is found.
    ///
    /// # Arguments
    ///
    /// * `process` - A reference to the `Process` in which the scan occurs.
    /// * `range` - A tuple containing:
    ///     - The starting address of the memory range
    ///     - The length of the memory range to scan
    pub async fn wait_scan(
        &self,
        process: &Process,
        range: (impl Into<Address>, u64),
    ) -> Address {
        let addr = range.0.into();
        retry(|| self.scan_process_range(process, (addr, range.1))).await
    }
}
