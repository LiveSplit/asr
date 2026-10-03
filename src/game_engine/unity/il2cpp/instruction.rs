//! Decodes how long an x86 or x64 instruction is and where control goes
//! after it. This is enough to walk the functions IL2CPP exports one real
//! instruction at a time, so a byte that only looks like a call inside an
//! operand is never taken for one.

/// Where control goes after an instruction. Targets are relative to the end
/// of the instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Flow {
    /// Goes on with the next instruction.
    Next,
    /// Goes on with the next instruction or jumps to the target.
    Branch(i32),
    /// Jumps to the target and never comes back.
    Jump(i32),
    /// Calls the target and goes on with the next instruction after it.
    Call(i32),
    /// Leaves the function, or jumps somewhere the code doesn't say.
    Stop,
}

/// An instruction's length and where control goes after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Instruction {
    pub(super) len: usize,
    pub(super) flow: Flow,
}

/// Builds a set of byte values from inclusive ranges.
const fn set(ranges: &[(u8, u8)]) -> [u32; 8] {
    let mut bits = [0; 8];
    let mut i = 0;
    while i < ranges.len() {
        let mut byte = ranges[i].0 as usize;
        while byte <= ranges[i].1 as usize {
            bits[byte / 32] |= 1 << (byte % 32);
            byte += 1;
        }
        i += 1;
    }
    bits
}

const fn has(bits: &[u32; 8], byte: u8) -> bool {
    bits[byte as usize / 32] & (1 << (byte % 32)) != 0
}

/// One-byte opcodes followed by a ModRM byte.
const MODRM: [u32; 8] = set(&[
    (0x00, 0x03),
    (0x08, 0x0B),
    (0x10, 0x13),
    (0x18, 0x1B),
    (0x20, 0x23),
    (0x28, 0x2B),
    (0x30, 0x33),
    (0x38, 0x3B),
    (0x62, 0x63),
    (0x69, 0x69),
    (0x6B, 0x6B),
    (0x80, 0x8F),
    (0xC0, 0xC1),
    (0xC4, 0xC7),
    (0xD0, 0xD3),
    (0xD8, 0xDF),
    (0xF6, 0xF7),
    (0xFE, 0xFF),
]);

/// One-byte opcodes with an 8-bit immediate.
const IMM8: [u32; 8] = set(&[
    (0x04, 0x04),
    (0x0C, 0x0C),
    (0x14, 0x14),
    (0x1C, 0x1C),
    (0x24, 0x24),
    (0x2C, 0x2C),
    (0x34, 0x34),
    (0x3C, 0x3C),
    (0x6A, 0x6A),
    (0x6B, 0x6B),
    (0x70, 0x7F),
    (0x80, 0x80),
    (0x82, 0x83),
    (0xA8, 0xA8),
    (0xB0, 0xB7),
    (0xC0, 0xC1),
    (0xC6, 0xC6),
    (0xCD, 0xCD),
    (0xD4, 0xD5),
    (0xE0, 0xE7),
    (0xEB, 0xEB),
]);

/// One-byte opcodes with a 16-bit or 32-bit immediate, by operand size.
const IMM_FULL: [u32; 8] = set(&[
    (0x05, 0x05),
    (0x0D, 0x0D),
    (0x15, 0x15),
    (0x1D, 0x1D),
    (0x25, 0x25),
    (0x2D, 0x2D),
    (0x35, 0x35),
    (0x3D, 0x3D),
    (0x68, 0x69),
    (0x81, 0x81),
    (0xA9, 0xA9),
    (0xB8, 0xBF),
    (0xC7, 0xC7),
    (0xE8, 0xE9),
]);

/// Two-byte opcodes, after `0F`, with no ModRM byte.
const PLAIN_0F: [u32; 8] = set(&[
    (0x05, 0x09),
    (0x0B, 0x0B),
    (0x30, 0x37),
    (0x77, 0x77),
    (0x80, 0x8F),
    (0xA0, 0xA2),
    (0xA8, 0xAA),
    (0xC8, 0xCF),
]);

/// Two-byte opcodes, after `0F`, with a ModRM byte and an 8-bit immediate.
const IMM8_0F: [u32; 8] = set(&[
    (0x0F, 0x0F),
    (0x70, 0x73),
    (0xA4, 0xA4),
    (0xAC, 0xAC),
    (0xBA, 0xBA),
    (0xC2, 0xC2),
    (0xC4, 0xC6),
]);

/// Decodes the instruction at the start of `code`. Returns `None` when the
/// bytes run out first, or for encodings IL2CPP's code doesn't use: VEX,
/// EVEX, the `67` prefix and far calls.
pub(super) fn decode(code: &[u8], x64: bool) -> Option<Instruction> {
    let mut at = 0;
    let mut operand16 = false;
    let mut rex = 0;
    loop {
        match *code.get(at)? {
            0x66 => {
                operand16 = true;
                rex = 0;
            }
            0x26 | 0x2E | 0x36 | 0x3E | 0x64 | 0x65 | 0xF0 | 0xF2 | 0xF3 => rex = 0,
            // The 67 prefix changes how the operand is addressed, which
            // IL2CPP code never does.
            0x67 => return None,
            // A REX byte only counts when the opcode comes right after it.
            byte @ 0x40..=0x4F if x64 => rex = byte,
            _ => break,
        }
        at += 1;
    }
    let wide = rex & 0x08 != 0;

    let opcode = *code.get(at)?;
    at += 1;
    let full = if operand16 && !wide { 2 } else { 4 };
    let mut second = 0;
    let (modrm, imm) = if opcode == 0x0F {
        second = *code.get(at)?;
        at += 1;
        match second {
            0x38 => {
                at += 1;
                (true, 0)
            }
            0x3A => {
                at += 1;
                (true, 1)
            }
            0x80..=0x8F => (false, 4),
            _ if has(&PLAIN_0F, second) => (false, 0),
            _ => (true, if has(&IMM8_0F, second) { 1 } else { 0 }),
        }
    } else {
        // Refuses VEX, EVEX and far calls and jumps. On x86, C4 and C5 are
        // LES and LDS when a memory operand follows, and VEX otherwise.
        let vex = match opcode {
            0xC4 | 0xC5 => x64 || *code.get(at)? >= 0xC0,
            0x62 => x64,
            _ => false,
        };
        if vex || matches!(opcode, 0x9A | 0xEA) {
            return None;
        }
        let imm = match opcode {
            // x64 ignores the 66 prefix on near calls and jumps.
            0xE8 | 0xE9 if x64 => 4,
            0xE8 | 0xE9 if operand16 => return None,
            0xB8..=0xBF if wide => 8,
            0xA0..=0xA3 => {
                if x64 {
                    8
                } else {
                    4
                }
            }
            0xC2 | 0xCA => 2,
            0xC8 => 3,
            _ if has(&IMM8, opcode) => 1,
            _ if has(&IMM_FULL, opcode) => full,
            _ => 0,
        };
        (has(&MODRM, opcode), imm)
    };

    let mut reg = 0;
    if modrm {
        let byte = *code.get(at)?;
        at += 1;
        let (mode, rm) = (byte >> 6, byte & 7);
        reg = (byte >> 3) & 7;
        if mode != 3 && rm == 4 {
            let sib = *code.get(at)?;
            at += 1;
            if mode == 0 && sib & 7 == 5 {
                at += 4;
            }
        }
        at += match (mode, rm) {
            (0, 5) | (2, _) => 4,
            (1, _) => 1,
            _ => 0,
        };
        // `test` with an immediate shares its opcode with `not`, `neg` and
        // the others, which take none.
        if matches!(opcode, 0xF6 | 0xF7) && reg < 2 {
            at += if opcode == 0xF6 { 1 } else { full };
        }
    }
    let end = at + imm;
    let len = if end <= code.len() { end } else { return None };

    let rel8 = || code[end - 1] as i8 as i32;
    let rel32 = || i32::from_le_bytes([code[end - 4], code[end - 3], code[end - 2], code[end - 1]]);
    let flow = match opcode {
        0x0F => match second {
            0x80..=0x8F => Flow::Branch(rel32()),
            0x0B => Flow::Stop,
            _ => Flow::Next,
        },
        0x70..=0x7F | 0xE0..=0xE3 => Flow::Branch(rel8()),
        0xE8 => Flow::Call(rel32()),
        0xE9 => Flow::Jump(rel32()),
        0xEB => Flow::Jump(rel8()),
        0xC2 | 0xC3 | 0xCA | 0xCB | 0xCC | 0xCF => Flow::Stop,
        0xFF if matches!(reg, 4 | 5) => Flow::Stop,
        _ => Flow::Next,
    };
    Some(Instruction { len, flow })
}
