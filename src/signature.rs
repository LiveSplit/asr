//! Support for finding patterns in a process's memory.

use crate::{Address, Process};
use core::{iter, mem, slice};

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
        /// Position of the primary exact-byte anchor (if any).
        anchor_pos: Option<Offset>,
        /// Byte value for the primary exact-byte anchor.
        anchor_byte: u8,
        /// Position of a secondary exact-byte check (if any).
        check_pos: Option<Offset>,
        /// Byte value for the secondary exact-byte check.
        check_byte: u8,
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

            let (anchor_pos, anchor_byte, check_pos, check_byte) =
                find_anchor_and_check(&needle, &mask);

            Self::Complex {
                needle,
                mask,
                anchor_pos,
                anchor_byte,
                check_pos,
                check_byte,
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

        iter::from_fn(move || {
            if cursor >= end {
                return None;
            }

            let found = match self {
                Signature::Simple(needle) => {
                    let check_pos = if N > 1 { Some(N - 1) } else { None };
                    let check_byte = check_pos.map_or(0, |i| needle[i]);
                    find_signature_from(
                        haystack,
                        cursor,
                        needle,
                        None,
                        Some(0),
                        needle[0],
                        check_pos,
                        check_byte,
                    )
                }
                Signature::Complex {
                    needle,
                    mask,
                    anchor_pos,
                    anchor_byte,
                    check_pos,
                    check_byte,
                } => find_signature_from(
                    haystack,
                    cursor,
                    needle,
                    Some(mask),
                    anchor_pos.map(|v| v as usize),
                    *anchor_byte,
                    check_pos.map(|v| v as usize),
                    *check_byte,
                ),
            };

            found.map(|start| {
                cursor = start.saturating_add(1);
                start
            })
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

        let mut addr: Address = range.0.into();
        let overall_end = addr.value() + range.1;

        // The sigscan essentially works by reading one memory page (0x1000 bytes)
        // at a time and looking for the signature in each page. We will create a buffer
        // sligthly larger than 0x1000 bytes in order to accomodate the size of
        // the memory page + the signature - 1. The very first bytes of the
        // buffer will be used as the tail of the previous memory page.
        // This allows to scan across the memory page boundaries.

        // The buffer struct is a convenience struct we want to reinterpret as an array
        // of u8 and the size of MEM_SIZE + N - 1
        #[repr(packed)]
        struct Buffer<const N: usize> {
            _head: [u8; N],
            _buffer: [u8; MEM_SIZE.saturating_sub(1)],
        }

        let mut last_page_success = false;

        // Although a bit slower, we need to ensure the compiler doesn't do unexpected optimizations
        // to the MaybeUninit struct. For this reason we are explicitly zero-initializing our buffer.
        // Using MaybeUninit here breaks the following code.
        // SAFETY: zero-initializing an array of u8 poses no problems in terms of memory safety
        let mut buffer = unsafe { mem::zeroed::<Buffer<N>>() };

        // SAFETY: As the data is zero-initialized, we know it's safe to reinterpret this data as
        // an array uf u8.
        let buffer = unsafe {
            slice::from_raw_parts_mut(&mut buffer as *mut _ as *mut u8, size_of::<Buffer<N>>())
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
                let (start, end) = buffer.split_at_mut(N.saturating_sub(1));
                start.copy_from_slice(&end[len.saturating_sub(N).saturating_add(1)..]);
            }

            let current_page_success = process
                .read_into_slice(addr, &mut buffer[N.saturating_sub(1)..][..len])
                .is_ok();

            // We define the final slice on which to perform the memory scan into. If we failed to read the memory page,
            // this returns an empty slice so the subsequent iterator will result into an empty iterator.
            // If we managed to read the current memory page, instead, we check if we have successfully read the data
            // from the previous memory page.
            let scan_buf = unsafe {
                if current_page_success {
                    if last_page_success {
                        slice::from_raw_parts(
                            buffer.as_ptr(),
                            len.saturating_add(N).saturating_sub(1),
                        )
                    } else {
                        slice::from_raw_parts(buffer.as_ptr().byte_add(N).byte_sub(1), len)
                    }
                } else {
                    &[]
                }
            };

            let cur_addr = addr;
            let cur_suc = last_page_success;

            addr = Address::new(end);
            last_page_success = current_page_success;

            Some(self.scan_internal(&scan_buf).map(move |pos| {
                let mut address = cur_addr.add(pos as u64);

                if cur_suc {
                    address = address.add_signed(-(N.saturating_sub(1) as i64))
                }

                address
            }))
        })
        .flatten()
    }
}

fn find_signature_from<const N: usize>(
    haystack: &[u8],
    start: usize,
    needle: &[u8; N],
    mask: Option<&[u8; N]>,
    anchor_pos: Option<usize>,
    anchor_byte: u8,
    check_pos: Option<usize>,
    check_byte: u8,
) -> Option<usize> {
    if haystack.len() < N {
        return None;
    }

    if start > haystack.len().saturating_sub(N) {
        return None;
    }

    if let Some(anchor_pos) = anchor_pos {
        let mut search_from = start.saturating_add(anchor_pos);

        while let Some(anchor_hit) = find_byte_swar(haystack, anchor_byte, search_from) {
            let match_start = anchor_hit.saturating_sub(anchor_pos);
            if match_start + N > haystack.len() {
                break;
            }

            if let Some(check_pos) = check_pos {
                if haystack[match_start + check_pos] != check_byte {
                    search_from = anchor_hit.saturating_add(1);
                    continue;
                }
            }

            if signature_matches_at(haystack, match_start, needle, mask) {
                return Some(match_start);
            }

            search_from = anchor_hit.saturating_add(1);
        }

        None
    } else {
        (start..=haystack.len() - N)
            .find(|&match_start| signature_matches_at(haystack, match_start, needle, mask))
    }
}

const fn find_anchor_and_check<const N: usize>(
    needle: &[u8; N],
    mask: &[u8; N],
) -> (Option<Offset>, u8, Option<Offset>, u8) {
    let mut anchor_pos = None;
    let mut anchor_byte = 0;

    let mut i = 0;
    while i < N {
        if mask[i] == 0xFF {
            anchor_pos = Some(i as Offset);
            anchor_byte = needle[i];
            break;
        }
        i += 1;
    }

    let mut check_pos = None;
    let mut check_byte = 0;

    if let Some(anchor) = anchor_pos {
        let anchor = anchor as usize;
        let mut farthest_distance = 0usize;
        let mut j = 0;
        while j < N {
            if j != anchor && mask[j] == 0xFF {
                let distance = if j > anchor { j - anchor } else { anchor - j };
                if distance > farthest_distance {
                    farthest_distance = distance;
                    check_pos = Some(j as Offset);
                    check_byte = needle[j];
                }
            }
            j += 1;
        }
    }

    (anchor_pos, anchor_byte, check_pos, check_byte)
}

#[inline]
fn signature_matches_at<const N: usize>(
    haystack: &[u8],
    start: usize,
    needle: &[u8; N],
    mask: Option<&[u8; N]>,
) -> bool {
    unsafe {
        let scan = haystack.as_ptr().add(start);
        match mask {
            None => exact_matches::<N>(scan, needle.as_ptr()),
            Some(mask) => masked_matches::<N>(scan, needle.as_ptr(), mask.as_ptr()),
        }
    }
}

#[inline]
unsafe fn exact_matches<const N: usize>(mut scan: *const u8, mut needle: *const u8) -> bool {
    let mut i = 0;

    #[cfg(target_feature = "simd128")]
    while i + 16 <= N {
        use core::arch::wasm32::{u8x16_ne, v128, v128_any_true};

        if v128_any_true(u8x16_ne(
            scan.cast::<v128>().read_unaligned(),
            needle.cast::<v128>().read_unaligned(),
        )) {
            return false;
        }

        scan = scan.add(16);
        needle = needle.add(16);
        i += 16;
    }

    while i + 8 <= N {
        if scan.cast::<u64>().read_unaligned() != needle.cast::<u64>().read_unaligned() {
            return false;
        }

        scan = scan.add(8);
        needle = needle.add(8);
        i += 8;
    }

    while i + 4 <= N {
        if scan.cast::<u32>().read_unaligned() != needle.cast::<u32>().read_unaligned() {
            return false;
        }

        scan = scan.add(4);
        needle = needle.add(4);
        i += 4;
    }

    while i + 2 <= N {
        if scan.cast::<u16>().read_unaligned() != needle.cast::<u16>().read_unaligned() {
            return false;
        }

        scan = scan.add(2);
        needle = needle.add(2);
        i += 2;
    }

    while i < N {
        if *scan != *needle {
            return false;
        }

        scan = scan.add(1);
        needle = needle.add(1);
        i += 1;
    }

    true
}

#[inline]
unsafe fn masked_matches<const N: usize>(
    mut scan: *const u8,
    mut needle: *const u8,
    mut mask: *const u8,
) -> bool {
    let mut i = 0;

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
        needle = needle.add(16);
        mask = mask.add(16);
        i += 16;
    }

    while i + 8 <= N {
        if scan.cast::<u64>().read_unaligned() & mask.cast::<u64>().read_unaligned()
            != needle.cast::<u64>().read_unaligned()
        {
            return false;
        }

        scan = scan.add(8);
        needle = needle.add(8);
        mask = mask.add(8);
        i += 8;
    }

    while i + 4 <= N {
        if scan.cast::<u32>().read_unaligned() & mask.cast::<u32>().read_unaligned()
            != needle.cast::<u32>().read_unaligned()
        {
            return false;
        }

        scan = scan.add(4);
        needle = needle.add(4);
        mask = mask.add(4);
        i += 4;
    }

    while i + 2 <= N {
        if scan.cast::<u16>().read_unaligned() & mask.cast::<u16>().read_unaligned()
            != needle.cast::<u16>().read_unaligned()
        {
            return false;
        }

        scan = scan.add(2);
        needle = needle.add(2);
        mask = mask.add(2);
        i += 2;
    }

    while i < N {
        if *scan & *mask != *needle {
            return false;
        }

        scan = scan.add(1);
        needle = needle.add(1);
        mask = mask.add(1);
        i += 1;
    }

    true
}

#[inline]
fn find_byte_swar(haystack: &[u8], needle: u8, mut start: usize) -> Option<usize> {
    if start >= haystack.len() {
        return None;
    }

    const ONES: u64 = 0x0101_0101_0101_0101;
    const HIGHS: u64 = 0x8080_8080_8080_8080;
    let repeated = (needle as u64) * ONES;

    let ptr = haystack.as_ptr();
    while start + 8 <= haystack.len() {
        let word = unsafe { ptr.add(start).cast::<u64>().read_unaligned() };
        let x = word ^ repeated;
        let eq = x.wrapping_sub(ONES) & !x & HIGHS;
        if eq != 0 {
            let index = (eq.trailing_zeros() / 8) as usize;
            return Some(start + index);
        }
        start += 8;
    }

    while start < haystack.len() {
        if haystack[start] == needle {
            return Some(start);
        }
        start += 1;
    }

    None
}
