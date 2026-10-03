//! Support for finding patterns in a process's memory.

use crate::{Address, Process};

/// A signature that can be used to find a pattern in a process's memory.
/// It is recommended to store this in a `static` or `const` variable to ensure
/// that the signature is parsed at compile time, which allows optimizations.
/// Also, compiling with the `simd128` feature is recommended for SIMD support.
///
/// The length is only used to size the signature. Every signature, whatever
/// its length, scans through the same code, so a splitter with signatures of
/// many lengths pays for one scanner.
#[derive(Debug, Clone, Copy)]
pub struct Signature<const N: usize> {
    /// The bytes to find, with the wildcard bits cleared.
    needle: [u8; N],
    /// The bits of each byte that have to match.
    mask: [u8; N],
    anchor: Anchor,
}

/// The two bytes a scan checks before it compares a whole signature. The
/// anchor is the byte with the most fixed bits, which the scan looks for
/// first. The check is the fixed byte farthest from the anchor.
#[derive(Debug, Clone, Copy)]
struct Anchor {
    pos: u8,
    byte: u8,
    mask: u8,
    /// The same as `pos` when the signature has no other fixed byte.
    check_pos: u8,
    check_byte: u8,
    check_mask: u8,
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

impl<const N: usize> Signature<N> {
    /// Creates a new signature from a string. The string must be a hexadecimal
    /// string with `?` as wildcard. It is recommended to store this in a
    /// `static` or `const` variable to ensure that the signature is parsed
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
        let mut parser = Parser {
            bytes: signature.as_bytes(),
        };
        let mut needle = [0; N];
        let mut mask = [0; N];
        let mut i = 0;
        loop {
            let (a, next) = parser.next();
            parser = next;
            let (b, next) = parser.next();
            parser = next;
            let (Some(a), Some(b)) = (a, b) else { break };
            assert!(i < N, "The signature is longer than its type says");
            mask[i] = ((a != 0x10) as u8 * 0xF0) | ((b != 0x10) as u8 * 0x0F);
            needle[i] = (a << 4) | (b & 0x0F);
            i += 1;
        }
        assert!(i == N, "The signature is shorter than its type says");
        Self::masked(needle, mask)
    }

    /// Creates a new signature from the bytes to find and a mask of the bits
    /// that have to match in each byte. This allows wildcards below the
    /// nibble that [`new`](Self::new) works with, such as for the fields of
    /// an ARM instruction.
    ///
    /// # Panics
    ///
    /// This function panics if the length is zero or exceeds 255 bytes.
    pub const fn masked(mut needle: [u8; N], mask: [u8; N]) -> Self {
        // The anchor stores positions in a byte.
        assert!(N > 0 && N < 256);
        let mut i = 0;
        while i < N {
            needle[i] &= mask[i];
            i += 1;
        }
        Self {
            needle,
            mask,
            anchor: Anchor::choose(&needle, &mask),
        }
    }

    #[inline]
    fn pattern(&self) -> Pattern<'_> {
        Pattern {
            needle: &self.needle,
            mask: &self.mask,
            anchor: self.anchor,
        }
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
    #[inline]
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
    #[inline]
    pub fn scan_iter<'a>(
        &'a self,
        process: &'a Process,
        range: (impl Into<Address>, u64),
    ) -> impl Iterator<Item = Address> + 'a {
        ScanIter::new(self.pattern(), process, range.0.into(), range.1)
    }
}

impl Anchor {
    const fn choose(needle: &[u8], mask: &[u8]) -> Self {
        let mut pos = 0;
        let mut i = 1;
        while i < needle.len() {
            if mask[i].count_ones() > mask[pos].count_ones() {
                pos = i;
            }
            i += 1;
        }
        let mut check_pos = pos;
        let mut i = 0;
        while i < needle.len() {
            if mask[i] != 0 && i.abs_diff(pos) > check_pos.abs_diff(pos) {
                check_pos = i;
            }
            i += 1;
        }
        Self {
            pos: pos as u8,
            byte: needle[pos],
            mask: mask[pos],
            check_pos: check_pos as u8,
            check_byte: needle[check_pos],
            check_mask: mask[check_pos],
        }
    }
}

/// A signature of any length. All the scanning works on this, so it is
/// compiled once rather than once per length.
#[derive(Clone, Copy)]
struct Pattern<'a> {
    needle: &'a [u8],
    mask: &'a [u8],
    anchor: Anchor,
}

impl Pattern<'_> {
    /// Finds the first match that starts at or after `from`.
    fn find(&self, haystack: &[u8], from: usize) -> Option<usize> {
        let n = self.needle.len();
        let last = haystack.len().checked_sub(n)?;
        if from > last {
            return None;
        }
        let anchor = self.anchor;
        let pos = anchor.pos as usize;
        // The anchor can't sit after the last start plus its position.
        let haystack_for_anchor = &haystack[..last + pos + 1];
        let mut search = from + pos;
        while let Some(hit) = find_byte(haystack_for_anchor, anchor.byte, anchor.mask, search) {
            let start = hit - pos;
            if haystack[start + anchor.check_pos as usize] & anchor.check_mask == anchor.check_byte
                && self.matches_at(&haystack[start..start + n])
            {
                return Some(start);
            }
            search = hit + 1;
        }
        None
    }

    /// Checks whether the signature matches these bytes, which are as many
    /// as the signature is long.
    #[inline]
    fn matches_at(&self, mut hay: &[u8]) -> bool {
        let mut needle = self.needle;
        let mut mask = self.mask;

        #[cfg(target_feature = "simd128")]
        while hay.len() >= 16 {
            use core::arch::wasm32::{u8x16_ne, v128, v128_and, v128_any_true};
            // SAFETY: Each slice holds at least 16 bytes, and v128 has no
            // alignment requirement for unaligned reads.
            let differs = unsafe {
                v128_any_true(u8x16_ne(
                    v128_and(
                        hay.as_ptr().cast::<v128>().read_unaligned(),
                        mask.as_ptr().cast::<v128>().read_unaligned(),
                    ),
                    needle.as_ptr().cast::<v128>().read_unaligned(),
                ))
            };
            if differs {
                return false;
            }
            hay = &hay[16..];
            needle = &needle[16..];
            mask = &mask[16..];
        }

        while let (Some((h, hr)), Some((n, nr)), Some((m, mr))) = (
            hay.split_first_chunk::<8>(),
            needle.split_first_chunk::<8>(),
            mask.split_first_chunk::<8>(),
        ) {
            if u64::from_ne_bytes(*h) & u64::from_ne_bytes(*m) != u64::from_ne_bytes(*n) {
                return false;
            }
            hay = hr;
            needle = nr;
            mask = mr;
        }

        hay.iter()
            .zip(needle)
            .zip(mask)
            .all(|((h, n), m)| h & m == *n)
    }
}

/// Finds the first byte at or after `from` that has `byte` in the bits of
/// `mask`, eight bytes at a time.
fn find_byte(haystack: &[u8], byte: u8, mask: u8, mut from: usize) -> Option<usize> {
    const ONES: u64 = 0x0101_0101_0101_0101;
    const HIGHS: u64 = 0x8080_8080_8080_8080;
    let bytes = byte as u64 * ONES;
    let masks = mask as u64 * ONES;
    while let Some(word) = haystack.get(from..from + 8) {
        let word = u64::from_le_bytes(word.try_into().ok()?);
        let differences = (word & masks) ^ bytes;
        // Flags the high bit of each zero byte. Bytes above the lowest zero
        // byte can be flagged wrongly, so only the lowest one is used.
        let zeros = differences.wrapping_sub(ONES) & !differences & HIGHS;
        if zeros != 0 {
            return Some(from + (zeros.trailing_zeros() / 8) as usize);
        }
        from += 8;
    }
    haystack
        .get(from..)?
        .iter()
        .position(|b| b & mask == byte)
        .map(|at| from + at)
}

/// A page of memory, which the scan reads at a time. A page is either
/// readable as a whole or not at all, so the reads stop at page boundaries.
const PAGE: usize = 0x1000;
/// Room in front of the page for the tail of the one before it, so a match
/// across the page boundary is seen. A signature is at most 255 bytes long.
const TAIL: usize = 0xFF;

struct ScanIter<'a> {
    pattern: Pattern<'a>,
    process: &'a Process,
    /// Where the next read starts.
    addr: u64,
    /// Where the range ends.
    end: u64,
    /// The bytes still to scan are `buf[lo..hi]`, of which `buf[lo..][..cursor]`
    /// have been searched already. The byte at `lo` is at address `base`.
    lo: usize,
    hi: usize,
    cursor: usize,
    base: u64,
    /// How many bytes of the tail of the last page sit in front of `TAIL`.
    carried: usize,
    buf: [u8; TAIL + PAGE],
}

impl<'a> ScanIter<'a> {
    fn new(pattern: Pattern<'a>, process: &'a Process, start: Address, len: u64) -> Self {
        Self {
            pattern,
            process,
            addr: start.value(),
            end: start.value().saturating_add(len),
            lo: 0,
            hi: 0,
            cursor: 0,
            base: 0,
            carried: 0,
            buf: [0; TAIL + PAGE],
        }
    }

    /// Reads the next page into the buffer and returns whether there was one.
    /// A page that can't be read leaves nothing to scan.
    fn read_page(&mut self) -> bool {
        if self.addr >= self.end {
            return false;
        }
        // Keep the tail of the bytes just scanned in front of the new page.
        // Those bytes are already scanned, so the matches they take part in
        // start inside them and end in the new page.
        let keep = (self.pattern.needle.len() - 1).min(self.hi - self.lo);
        self.buf.copy_within(self.hi - keep..self.hi, TAIL - keep);
        self.carried = keep;

        let page_end = ((self.addr & !(PAGE as u64 - 1)) + PAGE as u64).min(self.end);
        let len = (page_end - self.addr) as usize;
        let read = self
            .process
            .read_into_slice(Address::new(self.addr), &mut self.buf[TAIL..TAIL + len])
            .is_ok();
        if read {
            self.lo = TAIL - self.carried;
            self.hi = TAIL + len;
            self.base = self.addr - self.carried as u64;
        } else {
            self.lo = 0;
            self.hi = 0;
        }
        self.cursor = 0;
        self.addr = page_end;
        true
    }
}

impl Iterator for ScanIter<'_> {
    type Item = Address;

    fn next(&mut self) -> Option<Address> {
        loop {
            if let Some(found) = self.pattern.find(&self.buf[self.lo..self.hi], self.cursor) {
                self.cursor = found + 1;
                return Some(Address::new(self.base + found as u64));
            }
            if !self.read_page() {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Signature;
    use crate::{runtime::mock::with_process, Address};

    const SIGNATURE: Signature<4> = Signature::new("AA BB CC DD");

    /// Two pages of zeros starting at 0x10000, with the signature at `at`.
    fn pages_with_signature_at(at: u64) -> [u8; 0x2000] {
        let mut memory = [0; 0x2000];
        let at = (at - 0x10000) as usize;
        memory[at..at + 4].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        memory
    }

    #[test]
    fn finds_a_match_in_a_range_that_ends_partway_through_a_page() {
        let memory = pages_with_signature_at(0x11008);
        with_process(&[(0x10000, &memory)], |process| {
            assert_eq!(
                SIGNATURE.scan_process_range(process, (Address::new(0x10FF0), 0x20)),
                Some(Address::new(0x11008))
            );
        });
    }

    #[test]
    fn finds_a_match_across_the_first_page_boundary() {
        let memory = pages_with_signature_at(0x10FFE);
        with_process(&[(0x10000, &memory)], |process| {
            assert_eq!(
                SIGNATURE.scan_process_range(process, (Address::new(0x10FF0), 0x1010)),
                Some(Address::new(0x10FFE))
            );
        });
    }

    #[test]
    fn finds_a_signature_masked_below_the_nibble() {
        // Only the top 3 bits of the first byte and the low bit of the last
        // byte have to match.
        const SIG: Signature<3> = Signature::masked([0xA0, 0x12, 0x01], [0xE0, 0xFF, 0x01]);
        let mut memory = [0; 0x1000];
        memory[0x40..0x43].copy_from_slice(&[0xBF, 0x12, 0x03]);
        with_process(&[(0x10000, &memory)], |process| {
            assert_eq!(
                SIG.scan_process_range(process, (Address::new(0x10000), 0x1000)),
                Some(Address::new(0x10040))
            );
        });
    }

    #[test]
    fn finds_no_match_in_bytes_before_a_short_first_chunk() {
        // The range starts one byte before a page end, so the first chunk
        // holds AA alone. The signature would match two bytes before the
        // range if those bytes got scanned.
        let mut memory = [0x11; 0x2000];
        memory[0xFFF] = 0xAA;
        memory[0x1000] = 0xBB;
        let signature: Signature<4> = Signature::new("00 00 AA BB");
        with_process(&[(0x10000, &memory)], |process| {
            assert_eq!(
                signature.scan_process_range(process, (Address::new(0x10FFF), 0x100)),
                None
            );
        });
    }
}
