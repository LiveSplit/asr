//! Compares the scanner with a brute-force search on random memory,
//! signatures and ranges.

use std::{string::String, vec::Vec};

use crate::{runtime::mock::with_process, signature::Signature, Address};

/// A xorshift64* generator, so every run sees the same cases.
struct Rng(u64);

impl Rng {
    const fn step(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    const fn below(&mut self, n: u64) -> u64 {
        self.step() % n
    }

    const fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    /// Picks one of a few byte values, so signatures match often.
    const fn byte(&mut self) -> u8 {
        const BYTES: [u8; 6] = [0x00, 0x48, 0x8B, 0x05, 0xAA, 0xC3];
        BYTES[self.below(BYTES.len() as u64) as usize]
    }
}

/// Builds a random signature string of `n` bytes with its needle and mask.
/// A quarter have no wildcards, a quarter wildcard a nibble of every byte,
/// and the rest mix fixed bytes with whole and half wildcards.
fn random_signature(rng: &mut Rng, n: usize) -> (String, Vec<u8>, Vec<u8>) {
    let kind = rng.below(4);
    let mut text = String::new();
    let mut needle = Vec::new();
    let mut mask = Vec::new();
    for _ in 0..n {
        let byte = rng.byte();
        let fixed = match kind {
            0 => (true, true),
            1 if rng.chance(50) => (true, false),
            1 => (false, true),
            _ if rng.chance(50) => (true, true),
            _ if rng.chance(60) => (false, false),
            _ if rng.chance(50) => (true, false),
            _ => (false, true),
        };
        for (fixed, nibble) in [(fixed.0, byte >> 4), (fixed.1, byte & 0xF)] {
            text.push(if fixed {
                char::from_digit(nibble as u32, 16).unwrap()
            } else {
                '?'
            });
        }
        text.push(' ');
        let bits = (fixed.0 as u8 * 0xF0) | (fixed.1 as u8 * 0x0F);
        mask.push(bits);
        needle.push(byte & bits);
    }
    (text, needle, mask)
}

/// Fills `len` random bytes and puts the signature in at a few places, with
/// random bits under the wildcards.
fn random_bytes(rng: &mut Rng, len: usize, needle: &[u8], mask: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
    if len >= needle.len() {
        for _ in 0..rng.below(8) {
            let at = rng.below((len - needle.len() + 1) as u64) as usize;
            for (i, (needle, mask)) in needle.iter().zip(mask).enumerate() {
                bytes[at + i] = needle | rng.byte() & !mask;
            }
        }
    }
    bytes
}

/// Lists every place in `bytes` where the signature matches.
fn brute_force(bytes: &[u8], needle: &[u8], mask: &[u8]) -> Vec<usize> {
    (0..(bytes.len() + 1).saturating_sub(needle.len()))
        .filter(|&at| (0..needle.len()).all(|i| bytes[at + i] & mask[i] == needle[i]))
        .collect()
}

/// Lays out 1 to 3 regions of memory just before 0x20000, so many ranges
/// cross the 64 KB chunk boundary there. Their starts and ends aren't page
/// aligned. Some touch, so a read across them still fails.
fn random_regions(rng: &mut Rng, needle: &[u8], mask: &[u8]) -> Vec<(u64, Vec<u8>)> {
    let mut regions = Vec::new();
    let mut at = 0x1E000 + rng.below(0x2000);
    for _ in 0..1 + rng.below(3) {
        // Most regions and gaps are a few pages long. Some span chunks.
        let (most_len, most_gap) = if rng.chance(20) {
            (0x24000, 0x12000)
        } else {
            (0x3000, 0x1800)
        };
        let len = 1 + rng.below(most_len) as usize;
        regions.push((at, random_bytes(rng, len, needle, mask)));
        at += len as u64;
        if rng.chance(70) {
            at += rng.below(most_gap);
        }
    }
    regions
}

/// Picks a range around the regions. Many ranges start or end partway
/// through a page, and some are shorter than the signature or empty.
fn random_range(rng: &mut Rng, regions: &[(u64, Vec<u8>)], n: u64) -> (u64, u64) {
    let first = regions[0].0;
    let (last, bytes) = regions.last().unwrap();
    let span = last + bytes.len() as u64 - first;
    let start = match rng.below(5) {
        0 => first,
        1 => (first + rng.below(span)) & !0xFFF,
        2 => ((first + rng.below(span)) | 0xFFF) - rng.below(n),
        _ => first - 0x100 + rng.below(span + 0x200),
    };
    let len = match rng.below(6) {
        0 => 0,
        1 => rng.below(n),
        2 => n + rng.below(0x40),
        3 => 0x1000 - (start & 0xFFF) + rng.below(n),
        _ => rng.below((first + span + 0x200).saturating_sub(start) + 1),
    };
    (start, len)
}

/// Lists every match in the range that the scan can see. The scan reads
/// whole pages, so a match counts only when every page it touches lies in
/// a single region.
fn expected(
    regions: &[(u64, Vec<u8>)],
    needle: &[u8],
    mask: &[u8],
    start: u64,
    len: u64,
) -> Vec<u64> {
    let end = start + len;
    let mut memory = Vec::new();
    let mut page = start;
    while page < end {
        let page_end = ((page & !0xFFF) + 0x1000).min(end);
        let bytes = regions.iter().find_map(|(from, bytes)| {
            bytes.get(page.checked_sub(*from)? as usize..(page_end - from) as usize)
        });
        match bytes {
            Some(bytes) => memory.extend(bytes.iter().map(|&b| Some(b))),
            None => memory.extend((page..page_end).map(|_| None)),
        }
        page = page_end;
    }
    let n = needle.len();
    (0..(memory.len() + 1).saturating_sub(n))
        .filter(|&at| (0..n).all(|i| memory[at + i].is_some_and(|b| b & mask[i] == needle[i])))
        .map(|at| start + at as u64)
        .collect()
}

fn scan_ranges<const N: usize>(rng: &mut Rng) -> usize {
    let mut matches = 0;
    for case in 0..200 {
        let (text, needle, mask) = random_signature(rng, N);
        let regions = random_regions(rng, &needle, &mask);
        let (start, len) = random_range(rng, &regions, N as u64);
        let signature = Signature::<N>::new(&text);
        let expected = expected(&regions, &needle, &mask, start, len);

        let regions: Vec<(u64, &[u8])> = regions.iter().map(|(at, b)| (*at, &b[..])).collect();
        let found: Vec<u64> = with_process(&regions, |process| {
            signature
                .scan_iter(process, (Address::new(start), len))
                .map(Address::value)
                .collect()
        });
        assert_eq!(
            found, expected,
            "{N} bytes, case {case}: {text:?} at {start:#x}+{len:#x}"
        );
        matches += found.len();
    }
    matches
}

fn scan_slices<const N: usize>(rng: &mut Rng) -> usize {
    let mut matches = 0;
    for case in 0..400 {
        let (text, needle, mask) = random_signature(rng, N);
        let len = rng.below(0x400) as usize;
        let bytes = random_bytes(rng, len, &needle, &mask);
        let signature = Signature::<N>::new(&text);
        let found: Vec<usize> = signature.scan_slice(&bytes).collect();
        assert_eq!(
            found,
            brute_force(&bytes, &needle, &mask),
            "{N} bytes, case {case}: {text:?}"
        );
        matches += found.len();
    }
    matches
}

#[test]
fn finds_the_same_matches_as_a_brute_force_search() {
    let mut rng = Rng(0xD1FF_5EED);
    let mut matches = 0;
    macro_rules! lengths {
        ($($n:literal)*) => {$(
            matches += scan_ranges::<$n>(&mut rng);
            matches += scan_slices::<$n>(&mut rng);
        )*};
    }
    lengths!(1 2 3 4 7 8 9 15 16 17 24 31 32 33 64 100 255);
    // Checks that the cases find enough matches to mean something.
    assert!(matches > 10_000, "{matches}");
}
