//! Support for finding patterns in a process's memory.

use core::{mem, slice};

use bytemuck::AnyBitPattern;

use crate::{Address, Process};

type Offset = u8;

/// A signature that can be used to find a pattern in a process. It is
/// recommended to store this in a `static` or `const` variable to ensure that
/// the signature is parsed at compile time, which enables the code to be
/// optimized a lot more. Additionally it is recommend to compile the code with
/// the `simd128` target feature to enable the use of SIMD instructions.
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
                    b'0'..=b'9' => b - b'0',
                    b'a'..=b'f' => b - b'a' + 0xA,
                    b'A'..=b'F' => b - b'A' + 0xA,
                    b'?' => 0x10,
                    b' ' | b'\r' | b'\n' | b'\t' => continue,
                    _ => panic!("Invalid byte"),
                }),
                self,
            );
        }
        (None, self)
    }
}

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
    /// `static` or `const` variable to ensure that the signature is parsed at
    /// compile time, which enables the code to be optimized a lot more.
    ///
    /// # Panics
    ///
    /// This function panics if the signature is invalid. It also panics if the
    /// signature is longer than 255 bytes.
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
            assert!(i == N);

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

    fn scan(&self, haystack: &[u8]) -> Option<usize> {
        match self {
            Signature::Simple(needle) => memchr::memmem::find(haystack, needle),
            Signature::Complex {
                needle,
                mask,
                skip_offsets,
            } => {
                let mut current = 0;
                let end = N - 1;
                while let Some(scan) = strip_pod::<[u8; N]>(&mut &haystack[current..]) {
                    if matches(scan, needle, mask) {
                        return Some(current);
                    }
                    let offset = skip_offsets[scan[end] as usize];
                    current += offset as usize;
                }
                None
            }
        }
    }

    /// Scans a process for the signature. This will scan the address range of
    /// the process given. If the signature is found, the address of the start
    /// of the signature is returned.
    pub fn scan_process_range(
        &self,
        process: &Process,
        (addr, len): (impl Into<Address>, u64),
    ) -> Option<Address> {
        const MEM_SIZE: usize = 0x1000;
        let mut addr: Address = Into::into(addr);
        let overall_end = addr.value() + len;

        // The sigscan essentially works by reading one memory page (0x1000 bytes)
        // at a time and looking for the signature in each page.
        // We create a buffer sligthly larger than 0x1000 bytes in order to keep
        // the very first bytes as the tail of the previous memory page.
        // This allows to scan across the memory page boundaries.
        #[repr(packed)]
        struct Buffer<const N: usize> {
            _head: [u8; N],
            _buffer: [u8; MEM_SIZE]
        }

        let buf = 
            // SAFETY: Using mem::zeroed is safe in this instance as the Buffer struct
            // only contains u8, for which zeroed data represent a valid bit pattern.
            unsafe {
                let mut global_buffer = mem::zeroed::<Buffer<N>>();
                slice::from_raw_parts_mut(&mut global_buffer as *mut _ as *mut u8, N + MEM_SIZE)
        };

        let first = {
            // SAFETY: The buffer is guaranteed to be initialized due
            // to using mem::zeroed() so we can safely return an u8 slice of it.
            unsafe { slice::from_raw_parts_mut(buf.as_mut_ptr(), N) }
        };

        let last = {
            // SAFETY: The buffer is guaranteed to be initialized due
            // to using mem::zeroed() so we can safely return an u8 slice of it.
            unsafe { slice::from_raw_parts_mut(buf.as_mut_ptr().add(MEM_SIZE), N) }
        };

        let mut last_page_success = false;
        while addr.value() < overall_end {
            // We round up to the 4 KiB address boundary as that's a single
            // page, which is safe to read either fully or not at all. We do
            // this to reduce the number of syscalls as much as possible, as the
            // syscall overhead is quite high.
            let end = (addr.value() & !((4 << 10) - 1)) + (4 << 10).min(overall_end);
            let len = end - addr.value();

            // If we read the previous memory page successfully, then we can copy the last
            // elements to the start of the buffer. Otherwise, we just fill it with zeroes.
            if last_page_success {
                first.copy_from_slice(last);
                last_page_success = false;
            } else {
                first.fill(0);
            }

            if process.read_into_buf(addr, &mut buf[N..][..len as usize]).is_ok() {
                last_page_success = true;
                if let Some(pos) = self.scan(&mut buf[..len as usize + N]) {
                    return Some(addr.add(pos as u64).add_signed(-(N as i64)));
                }
            };
            addr = Address::new(end);
        }
        None
    }
}

fn matches<const N: usize>(scan: &[u8; N], needle: &[u8; N], mask: &[u8; N]) -> bool {
    // SAFETY: Before reading individual chunks from the arrays, we check that
    // we can still read values of that size. We also read them unaligned as the
    // original arrays are entirely unaligned.
    unsafe {
        let mut i = 0;
        let (mut scan, mut needle, mut mask) = (scan.as_ptr(), needle.as_ptr(), mask.as_ptr());
        #[cfg(target_feature = "simd128")]
        while i + 16 <= N {
            use core::arch::wasm32::{u8x16_ne, v128, v128_and, v128_any_true};

            if v128_any_true(u8x16_ne(
                v128_and(
                    scan.cast::<v128>().read_unaligned(),
                    mask.cast::<v128>().read_unaligned(),
                ),
                needle.cast::<v128>().read_unaligned(),
            )) {
                return false;
            }
            scan = scan.add(16);
            mask = mask.add(16);
            needle = needle.add(16);
            i += 16;
        }
        while i + 8 <= N {
            if scan.cast::<u64>().read_unaligned() & mask.cast::<u64>().read_unaligned()
                != needle.cast::<u64>().read_unaligned()
            {
                return false;
            }
            scan = scan.add(8);
            mask = mask.add(8);
            needle = needle.add(8);
            i += 8;
        }
        while i + 4 <= N {
            if scan.cast::<u32>().read_unaligned() & mask.cast::<u32>().read_unaligned()
                != needle.cast::<u32>().read_unaligned()
            {
                return false;
            }
            scan = scan.add(4);
            mask = mask.add(4);
            needle = needle.add(4);
            i += 4;
        }
        while i + 2 <= N {
            if scan.cast::<u16>().read_unaligned() & mask.cast::<u16>().read_unaligned()
                != needle.cast::<u16>().read_unaligned()
            {
                return false;
            }
            scan = scan.add(2);
            mask = mask.add(2);
            needle = needle.add(2);
            i += 2;
        }
        while i < N {
            if *scan & *mask != *needle {
                return false;
            }
            scan = scan.add(1);
            mask = mask.add(1);
            needle = needle.add(1);
            i += 1;
        }
        true
    }
}

fn strip_pod<'a, T: AnyBitPattern>(cursor: &mut &'a [u8]) -> Option<&'a T> {
    if cursor.len() < mem::size_of::<T>() {
        return None;
    }
    let (before, after) = cursor.split_at(mem::size_of::<T>());
    *cursor = after;
    Some(bytemuck::from_bytes(before))
}
