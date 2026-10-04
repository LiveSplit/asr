//! Support for finding patterns in a process's memory.

use crate::{Address, Process};

/// A signature that can be used to find a pattern in a process's memory.
/// It is recommended to store this in a `static` or `const` variable to ensure
/// that the signature is parsed at compile time, which allows optimizations.
/// Also, compiling with the `simd128` feature is recommended for SIMD support.
///
/// The length is only used to size the signature. Every signature, whatever
/// its length, scans through the same code, so a splitter with signatures of
/// many lengths pays for 1 scanner.
#[derive(Debug, Clone, Copy)]
pub struct Signature<const N: usize> {
    /// The bytes to find, with the wildcard bits cleared.
    needle: [u8; N],
    /// The bits of each byte that have to match.
    mask: [u8; N],
    anchor: Anchor,
}

/// Holds the 2 bytes a scan checks before it compares a whole signature:
/// the 2 rarest fixed bytes, going by how often each byte value shows up
/// in x64 code. A byte with wildcard bits only gets picked when there are
/// fewer than 2 fixed bytes. The scan looks for the anchor first and checks
/// the check byte at each hit. When every other byte is a full wildcard,
/// the check is the anchor again.
#[derive(Debug, Clone, Copy)]
struct Anchor {
    pos: u8,
    byte: u8,
    mask: u8,
    check_pos: u8,
    check_byte: u8,
    check_mask: u8,
}

/// Ranks each byte value by how common it is in x64 code, from 0 for the
/// rarest to 255 for the most common. Counted over the modules of a Unity 6
/// IL2CPP player (GameAssembly.dll and UnityPlayer.dll, 47 MB). Zero, the
/// REX prefixes and `mov` are the most common; a scan anchored on them
/// checks a candidate every few bytes. The table is only read while a
/// signature is built, so a `const` signature never carries it into the
/// auto splitter.
#[rustfmt::skip]
const RANK: [u8; 256] = [
    255, 251, 237, 226, 225, 227, 179, 173, 234, 159, 161, 146, 195, 215, 139, 252,
    241, 220, 122, 124, 153, 180, 103, 105, 212,  89,  61,  77, 121,  97,  53, 168,
    239, 163,  55,  54, 249,  84,  46,  41, 229, 156,  39, 140,  87,  74, 132,  81,
    223,  47,  90, 209, 129,  91,  26,  24, 208, 148, 130, 204, 114, 116,  49,  88,
    230, 245, 158, 194, 243, 233, 142, 162, 254, 236,  95, 136, 242, 214, 117, 127,
    218,  71, 101, 144, 193, 176, 133, 187, 205, 196,  15, 125, 210, 138, 123, 181,
    188, 171,  57, 183, 178, 216, 222, 128, 169, 199,  28,  72, 190, 113, 202, 186,
    207,  12, 201, 189, 240, 224, 115, 102, 175, 137,  25,  64, 165, 110,  83, 118,
    231, 200, 119, 238, 221, 228,  92, 106, 182, 247,  37, 253, 108, 246,  45,  62,
    172,  11,  21,  18,  76,  42,   3,  27, 134,   8,   1,   2,  78,   7,   0,   6,
    157,  20,  32,  44,  68,  29,  17,  59, 147,  30,  67,  43,  85,  33,  23,  75,
    151,  22,   4,   5,  93,  14, 152, 155, 177, 112,  96,  36,  79,  38,  51,  58,
    235, 219, 166, 213, 197, 100, 203, 211, 217, 184, 111, 174, 248,  86, 150, 164,
    206, 143, 154, 135,  66,  60, 126, 131, 167, 109,  63, 104,  34,   9,  31,  48,
    185,  99,  73,  50, 107,  10,  13,  35, 244, 170,  56, 198, 160,  52,  16,  40,
    191,  98, 149, 232,  65,  19, 145,  94, 192, 120,  82,  70,  80,  69, 141, 250,
];

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
    /// This function panics if the signature is invalid or empty, or if its
    /// length exceeds 255 bytes.
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
    const fn pattern(&self) -> Pattern<'_> {
        Pattern::new(&self.needle, &self.mask, self.anchor)
    }

    /// Returns an iterator over the positions in the slice where the
    /// signature matches. Use this on memory that is already read, such as
    /// the bytes of a function.
    #[inline]
    pub fn scan_slice<'a>(&'a self, haystack: &'a [u8]) -> impl Iterator<Item = usize> + 'a {
        SliceIter {
            pattern: self.pattern(),
            haystack,
            cursor: 0,
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
    /// Rates how rare a byte of the signature is: a fixed byte goes by the
    /// table, a byte with wildcard bits is worse than any fixed one, and the
    /// fewer bits it fixes, the worse it is.
    const fn rarity(needle: u8, mask: u8) -> u16 {
        if mask == 0xFF {
            RANK[needle as usize] as u16
        } else {
            0x100 + (8 - mask.count_ones()) as u16
        }
    }

    /// Picks the rarest byte as the anchor and the second rarest as the
    /// check.
    const fn choose(needle: &[u8], mask: &[u8]) -> Self {
        let mut best = 0;
        let mut second = 0;
        if needle.len() > 1 {
            second = 1;
            if Self::rarity(needle[1], mask[1]) < Self::rarity(needle[0], mask[0]) {
                (best, second) = (1, 0);
            }
        }
        let mut i = 2;
        while i < needle.len() {
            let rarity = Self::rarity(needle[i], mask[i]);
            if rarity < Self::rarity(needle[best], mask[best]) {
                second = best;
                best = i;
            } else if rarity < Self::rarity(needle[second], mask[second]) {
                second = i;
            }
            i += 1;
        }
        // A check that is a pure wildcard is no check.
        if mask[second] == 0 {
            second = best;
        }
        Self {
            pos: best as u8,
            byte: needle[best],
            mask: mask[best],
            check_pos: second as u8,
            check_byte: needle[second],
            check_mask: mask[second],
        }
    }
}

/// A signature of any length. All the scanning works on this, so it is
/// compiled once rather than once per length.
/// The needle and mask always have equal lengths, checked by `new`.
#[derive(Clone, Copy)]
struct Pattern<'a> {
    needle: &'a [u8],
    mask: &'a [u8],
    anchor: Anchor,
}

impl<'a> Pattern<'a> {
    #[inline]
    const fn new(needle: &'a [u8], mask: &'a [u8], anchor: Anchor) -> Self {
        assert!(
            needle.len() == mask.len(),
            "needle and mask lengths must match"
        );
        Self {
            needle,
            mask,
            anchor,
        }
    }

    /// Finds the first match that starts at or after `from`.
    fn find(&self, haystack: &[u8], from: usize) -> Option<usize> {
        let last = haystack.len().checked_sub(self.needle.len())?;
        #[cfg(target_feature = "simd128")]
        let from = match self.find_simd(haystack, from, last) {
            Ok(found) => return Some(found),
            Err(from) => from,
        };
        if from > last {
            return None;
        }
        self.find_anchor(haystack, from, last)
    }

    /// Looks at 16 starts at a time while there are that many. Returns the
    /// first match, or as the error the start to go on from.
    #[cfg(target_feature = "simd128")]
    fn find_simd(&self, haystack: &[u8], mut from: usize, last: usize) -> Result<usize, usize> {
        use core::arch::wasm32::{i8x16_splat, u8x16_bitmask, u8x16_eq, v128, v128_and};
        let anchor = self.anchor;
        let (pos, check_pos) = (anchor.pos as usize, anchor.check_pos as usize);
        // Compares the anchor bytes and the check bytes of 16 starts at once,
        // and only looks at the starts where both are right.
        let anchor_bytes = i8x16_splat(anchor.byte as i8);
        let anchor_masks = i8x16_splat(anchor.mask as i8);
        let check_bytes = i8x16_splat(anchor.check_byte as i8);
        let check_masks = i8x16_splat(anchor.check_mask as i8);
        while from + pos.max(check_pos) + 16 <= haystack.len() && from <= last {
            // SAFETY: Both loads end at or before the end of the haystack.
            let (anchors, checks) = unsafe {
                let base = haystack.as_ptr().add(from);
                (
                    base.add(pos).cast::<v128>().read_unaligned(),
                    base.add(check_pos).cast::<v128>().read_unaligned(),
                )
            };
            let mut candidates = u8x16_bitmask(v128_and(
                u8x16_eq(v128_and(anchors, anchor_masks), anchor_bytes),
                u8x16_eq(v128_and(checks, check_masks), check_bytes),
            ));
            while candidates != 0 {
                let start = from + candidates.trailing_zeros() as usize;
                if start > last {
                    return Err(last + 1);
                }
                let candidate = &haystack[start..start + self.needle.len()];
                // SAFETY: The checked slice contains exactly needle.len() bytes.
                if unsafe { self.matches_at(candidate) } {
                    return Ok(start);
                }
                candidates &= candidates - 1;
            }
            from += 16;
        }
        Err(from)
    }

    /// Finds the first match at or after `from`. Looks for the anchor byte
    /// 32 bytes at a time, as 4 words with 1 branch for all 4, since most
    /// steps find nothing. On a step with hits, drops the hits whose check
    /// byte is wrong, 32 at a time too, and compares the signature at the
    /// rest. So an anchor that hits every few bytes, such as a nibble, stays
    /// in the fast loop.
    fn find_anchor(&self, haystack: &[u8], from: usize, last: usize) -> Option<usize> {
        const ONES: u64 = 0x0101_0101_0101_0101;
        let anchor = self.anchor;
        let (pos, check_pos) = (anchor.pos as usize, anchor.check_pos as usize);
        let (bytes, masks) = (anchor.byte as u64 * ONES, anchor.mask as u64 * ONES);
        let (check_bytes, check_masks) = (
            anchor.check_byte as u64 * ONES,
            anchor.check_mask as u64 * ONES,
        );
        // Flags the bytes that are `bytes` in the 32 bytes of `hay` at `at`,
        // 1 bit per byte, or returns `None` where fewer bytes are left.
        let hits = |hay: &[u8], at: usize, bytes, masks| {
            let (words, _) = hay.get(at..at + 32)?.as_chunks::<8>();
            let mut bits = 0;
            for (i, word) in words.iter().enumerate() {
                bits |= bits_of(word_hits(*word, bytes, masks)) << (i * 8);
            }
            Some(bits)
        };
        let matches = |start: usize| {
            haystack[start + check_pos] & anchor.check_mask == anchor.check_byte && {
                let candidate = &haystack[start..start + self.needle.len()];
                // SAFETY: The checked slice contains exactly needle.len() bytes.
                unsafe { self.matches_at(candidate) }
            }
        };
        // The anchor can't sit after the last start plus its position.
        let hay = &haystack[..last + pos + 1];
        let mut at = from + pos;
        while let Some(mut candidates) = hits(hay, at, bytes, masks) {
            let base = at - pos;
            if candidates != 0 {
                if let Some(checks) = hits(haystack, base + check_pos, check_bytes, check_masks) {
                    candidates &= checks;
                }
            }
            while candidates != 0 {
                let start = base + candidates.trailing_zeros() as usize;
                if matches(start) {
                    return Some(start);
                }
                candidates &= candidates - 1;
            }
            at += 32;
        }
        (at..hay.len())
            .find(|&at| hay[at] & anchor.mask == anchor.byte && matches(at - pos))
            .map(|at| at - pos)
    }

    /// Checks whether the signature matches these bytes, which are as many
    /// as the signature is long. Only starts whose anchor and check bytes
    /// match get here. Compare whole words before the remaining bytes.
    ///
    /// # Safety
    ///
    /// `hay` must have the same length as `self.needle`.
    // An inline hint can duplicate the verifier in both candidate paths.
    unsafe fn matches_at(&self, hay: &[u8]) -> bool {
        debug_assert!(hay.len() == self.needle.len() && hay.len() == self.mask.len());
        // SAFETY: The caller guarantees hay.len() == needle.len(), and
        // Pattern::new checks that the needle and mask have equal lengths.
        // Exposing this invariant to the optimizer removes redundant bounds
        // checks and zip length comparisons without checking each candidate.
        unsafe {
            core::hint::assert_unchecked(
                hay.len() == self.needle.len() && hay.len() == self.mask.len(),
            );
        }
        // SAFETY: The caller's length guarantee and Pattern's invariant give
        // all three slices the same length.
        #[cfg(target_feature = "simd128")]
        let Some(checked) = (unsafe { self.matches_simd(hay) }) else {
            return false;
        };
        #[cfg(target_feature = "simd128")]
        if checked == hay.len() {
            return true;
        }
        #[cfg(not(target_feature = "simd128"))]
        let checked = 0;
        // Chunking the slices gives word-sized comparisons without unaligned
        // pointer reads or a separate scanner for each signature length.
        let (hay, tail) = hay[checked..].as_chunks::<8>();
        let (needle, needle_tail) = self.needle[checked..].as_chunks::<8>();
        let (mask, mask_tail) = self.mask[checked..].as_chunks::<8>();
        if !hay.iter().zip(needle).zip(mask).all(|((h, n), m)| {
            u64::from_ne_bytes(*h) & u64::from_ne_bytes(*m) == u64::from_ne_bytes(*n)
        }) {
            return false;
        }
        tail.iter()
            .zip(needle_tail)
            .zip(mask_tail)
            .all(|((h, n), m)| h & m == *n)
    }

    /// Compares 16 bytes at a time while there are that many. Returns how
    /// many bytes it compared, or `None` when one differs.
    ///
    /// # Safety
    ///
    /// `hay` must have the same length as `self.needle`.
    #[cfg(target_feature = "simd128")]
    unsafe fn matches_simd(&self, hay: &[u8]) -> Option<usize> {
        use core::arch::wasm32::{u8x16_ne, v128, v128_and, v128_any_true};
        let mut checked = 0;
        while checked + 16 <= hay.len() {
            // SAFETY: The caller's length guarantee and Pattern's invariant
            // give all three slices the same length. The loop checks that
            // 16 bytes remain; read_unaligned does not require v128 alignment.
            let differs = unsafe {
                let load =
                    |bytes: &[u8]| bytes.as_ptr().add(checked).cast::<v128>().read_unaligned();
                v128_any_true(u8x16_ne(
                    v128_and(load(hay), load(self.mask)),
                    load(self.needle),
                ))
            };
            if differs {
                return None;
            }
            checked += 16;
        }
        Some(checked)
    }
}

/// Flags the bytes of a word that are `bytes` in the bits of `masks` by
/// setting their high bit. A byte above a flagged one can get flagged too,
/// and the full compare after it drops those.
const fn word_hits(word: [u8; 8], bytes: u64, masks: u64) -> u64 {
    const ONES: u64 = 0x0101_0101_0101_0101;
    const HIGHS: u64 = 0x8080_8080_8080_8080;
    let differences = (u64::from_le_bytes(word) & masks) ^ bytes;
    differences.wrapping_sub(ONES) & !differences & HIGHS
}

/// Packs the high bit of each byte into 1 bit per byte. The multiply adds
/// up the word shifted left by 0, 7, 14 and so on up to 49 bits. That puts
/// the high bit of byte 0 at bit 56, of byte 1 at bit 57 and so on, and
/// nothing else reaches the top byte. Hits in bytes 0 and 2,
/// `0x0000_0000_0080_0080`, give `0b101`.
const fn bits_of(hits: u64) -> u32 {
    (hits.wrapping_mul(0x0002_0408_1020_4081) >> 56) as u32
}

struct SliceIter<'a> {
    pattern: Pattern<'a>,
    haystack: &'a [u8],
    cursor: usize,
}

impl Iterator for SliceIter<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        let found = self.pattern.find(self.haystack, self.cursor)?;
        self.cursor = found + 1;
        Some(found)
    }
}

/// The size of a page of memory. A page is readable as a whole or not at
/// all, so a read never straddles one that isn't.
const PAGE: usize = 0x1000;
/// How much the scan reads at a time where it can. Each read is a call into
/// the host, so the scan reads many pages at once and goes 1 page at a time
/// only inside a chunk that failed.
const CHUNK: usize = 0x10000;
/// Room in front of the chunk for the tail of the one before it, so a match
/// across the 2 chunks is seen. A signature is at most 255 bytes long.
const TAIL: usize = 0xFF;

struct ScanIter<'a> {
    pattern: Pattern<'a>,
    process: &'a Process,
    /// Where the next read starts.
    addr: u64,
    /// Where the range ends.
    end: u64,
    /// Up to where the reads go 1 page at a time, after a chunk failed.
    paged_until: u64,
    /// The bytes to scan are `buf[lo..hi]`, of which the first `cursor` are
    /// searched already. The byte at `lo` is at address `base`.
    lo: usize,
    hi: usize,
    cursor: usize,
    base: u64,
    /// How many bytes of the tail of the last read sit in front of `TAIL`.
    carried: usize,
    buf: [u8; TAIL + CHUNK],
}

impl<'a> ScanIter<'a> {
    const fn new(pattern: Pattern<'a>, process: &'a Process, start: Address, len: u64) -> Self {
        Self {
            pattern,
            process,
            addr: start.value(),
            end: start.value().saturating_add(len),
            paged_until: 0,
            lo: 0,
            hi: 0,
            cursor: 0,
            base: 0,
            carried: 0,
            buf: [0; TAIL + CHUNK],
        }
    }

    /// Reads the next chunk, or page inside a failed chunk, into the buffer
    /// and returns whether there was one. Memory that can't be read leaves
    /// nothing to scan.
    fn read_next(&mut self) -> bool {
        if self.addr >= self.end {
            return false;
        }
        // Keep the tail of the bytes just scanned in front of the new ones,
        // for the matches that start in the old bytes and end in the new.
        let keep = (self.pattern.needle.len() - 1).min(self.hi - self.lo);
        self.buf.copy_within(self.hi - keep..self.hi, TAIL - keep);
        self.carried = keep;

        let (read, read_end) = loop {
            let step = if self.addr < self.paged_until {
                PAGE
            } else {
                CHUNK
            } as u64;
            let step_end = (self.addr & !(step - 1)).saturating_add(step).min(self.end);
            let len = (step_end - self.addr) as usize;
            let read = self
                .process
                .read_into_slice(Address::new(self.addr), &mut self.buf[TAIL..TAIL + len])
                .is_ok();
            if read || step == PAGE as u64 {
                break (read, step_end);
            }
            // Some page of the chunk can't be read. Find out which can.
            self.paged_until = step_end;
        };
        if read {
            self.lo = TAIL - self.carried;
            self.hi = TAIL + (read_end - self.addr) as usize;
            self.base = self.addr - self.carried as u64;
        } else {
            self.lo = 0;
            self.hi = 0;
        }
        self.cursor = 0;
        self.addr = read_end;
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
            if !self.read_next() {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Anchor, Pattern, Signature};
    use crate::{runtime::mock::with_process, Address};

    const SIGNATURE: Signature<4> = Signature::new("AA BB CC DD");

    #[test]
    #[should_panic(expected = "needle and mask lengths must match")]
    fn rejects_a_mask_shorter_than_the_needle() {
        Pattern::new(&[0xAA; 2], &[0xFF], Anchor::choose(&[0xAA], &[0xFF]));
    }

    #[test]
    #[should_panic(expected = "needle and mask lengths must match")]
    fn rejects_a_mask_longer_than_the_needle() {
        Pattern::new(&[0xAA], &[0xFF; 2], Anchor::choose(&[0xAA], &[0xFF]));
    }

    #[test]
    fn full_comparisons_check_every_cared_bit_at_word_and_tail_boundaries() {
        // Test the verifier directly so the anchor cannot reject a mismatch
        // before the word / tail comparison runs. The haystack is unaligned.
        for len in 1..=255 {
            for kind in 0..3 {
                let mut needle = [0; 255];
                let mut mask = [0; 255];
                let mut bytes = [0; 256];
                for i in 0..len {
                    mask[i] = match kind {
                        0 => 0,
                        1 => 0xFF,
                        _ => (i as u8).wrapping_mul(37).wrapping_add(0xA5),
                    };
                    needle[i] = (i as u8).wrapping_mul(57) & mask[i];
                    bytes[i + 1] = needle[i] | (0x5A & !mask[i]);
                }
                let pattern = Pattern::new(
                    &needle[..len],
                    &mask[..len],
                    Anchor::choose(&needle[..len], &mask[..len]),
                );
                let matches = |bytes: &[u8]| {
                    let candidate = &bytes[1..=len];
                    // SAFETY: Both the candidate and the needle have len bytes.
                    unsafe { pattern.matches_at(candidate) }
                };
                assert!(matches(&bytes));
                for i in 0..len {
                    for bit in 0..8 {
                        let cared = mask[i] & (1 << bit);
                        if cared != 0 {
                            bytes[i + 1] ^= cared;
                            assert!(!matches(&bytes), "len={len}, byte={i}, bit={bit}");
                            bytes[i + 1] ^= cared;
                        }
                    }
                    let ignored = !mask[i];
                    bytes[i + 1] ^= ignored;
                    assert!(matches(&bytes), "len={len}, byte={i}");
                    bytes[i + 1] ^= ignored;
                }
            }
        }
    }

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
        // The range starts 1 byte before a page end, so the first chunk
        // holds AA alone. The signature would match 2 bytes before the
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

    #[test]
    fn scans_a_slice() {
        const SIG: Signature<3> = Signature::new("A? ?? 0B");
        let haystack = [0xA1, 0x00, 0x0B, 0xA2, 0xFF, 0x1B, 0xAF, 0x12, 0x0B];
        let found: std::vec::Vec<usize> = SIG.scan_slice(&haystack).collect();
        assert_eq!(found, [0, 6]);
    }

    #[test]
    fn scans_a_slice_without_a_fixed_byte() {
        const SIG: Signature<2> = Signature::new("?1 2?");
        let haystack = [0x01, 0x20, 0x00, 0x11, 0x2F, 0x21, 0x21];
        let found: std::vec::Vec<usize> = SIG.scan_slice(&haystack).collect();
        assert_eq!(found, [0, 3, 5]);
    }

    #[test]
    fn reads_many_pages_at_once() {
        let memory = std::vec![0; 0x10000];
        with_process(&[(0x10000, &memory)], |process| {
            assert_eq!(
                SIGNATURE
                    .scan_iter(process, (Address::new(0x10000), 0x10000))
                    .count(),
                0
            );
            assert_eq!(crate::runtime::mock::reads(), 1);
        });
    }

    #[test]
    fn finds_a_match_after_a_page_that_cannot_be_read() {
        // The second page of the range can't be read, so the scan goes on
        // page by page and still finds the match further on.
        let first = [0; 0x1000];
        let mut rest = std::vec![0; 0xE000];
        rest[0x3000..0x3004].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        with_process(&[(0x10000, &first), (0x12000, &rest)], |process| {
            assert_eq!(
                SIGNATURE.scan_process_range(process, (Address::new(0x10000), 0x10000)),
                Some(Address::new(0x15000))
            );
        });
    }

    #[test]
    fn reads_page_by_page_only_inside_a_chunk_that_failed() {
        // The first chunk has a page that can't be read, so it takes 1 read
        // for the chunk and 16 for its pages. The next 2 chunks take 1 read
        // each, and the match across them is still found.
        let first = [0; 0x1000];
        let mut rest = std::vec![0; 0x2E000];
        rest[0x1DFFE..0x1E002].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        with_process(&[(0x10000, &first), (0x12000, &rest)], |process| {
            let found: std::vec::Vec<Address> = SIGNATURE
                .scan_iter(process, (Address::new(0x10000), 0x30000))
                .collect();
            assert_eq!(found, [Address::new(0x2FFFE)]);
            assert_eq!(crate::runtime::mock::reads(), 19);
        });
    }

    #[test]
    fn stops_at_the_end_of_the_address_space() {
        // The range runs past the last page, so the scan must not wrap
        // around to the match at the start of memory.
        let last = [0; 0x1000];
        let first = [0xAA, 0xBB, 0xCC, 0xDD];
        with_process(
            &[(0xFFFF_FFFF_FFFF_F000, &last), (0x10000, &first)],
            |process| {
                assert_eq!(
                    SIGNATURE
                        .scan_process_range(process, (Address::new(0xFFFF_FFFF_FFFF_F000), 0x2000)),
                    None
                );
            },
        );
    }

    #[test]
    fn anchors_on_the_rarest_fixed_bytes() {
        // 48 and 8B are the most common bytes in x64 code, 3C is rare.
        const SIG: Signature<12> = Signature::new("48 8B 05 ?? ?? ?? ?? 48 83 3C ?? 00");
        assert_eq!(SIG.anchor.pos, 9);
        assert_eq!(SIG.anchor.byte, 0x3C);
        assert_eq!(SIG.anchor.check_pos, 2);
        assert_eq!(SIG.anchor.check_byte, 0x05);
    }

    #[test]
    fn a_wildcard_is_no_check() {
        const SIG: Signature<4> = Signature::new("A3 ?? ?? ??");
        assert_eq!(SIG.anchor.pos, 0);
        assert_eq!(SIG.anchor.check_pos, 0);
    }
}
