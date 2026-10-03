//! Tests the instruction decoder on encodings taken from the code IL2CPP
//! players run. Every length here was checked against a full disassembler.

use super::instruction::{decode, Flow};

fn bytes(hex: &str) -> std::vec::Vec<u8> {
    hex.split_whitespace()
        .map(|byte| u8::from_str_radix(byte, 16).unwrap())
        .collect()
}

fn length(hex: &str, x64: bool) -> usize {
    decode(&bytes(hex), x64).unwrap().len
}

fn flow(hex: &str, x64: bool) -> Flow {
    decode(&bytes(hex), x64).unwrap().flow
}

#[test]
fn measures_x64_instructions() {
    for (hex, len) in [
        ("48 8B 05 11 22 33 44", 7),
        ("48 83 EC 28", 4),
        ("40 53", 2),
        ("48 89 5C 24 08", 5),
        ("48 8B 84 C8 10 00 00 00", 8),
        ("66 0F 1F 44 00 00", 6),
        ("0F 1F 80 00 00 00 00", 7),
        ("48 B8 11 22 33 44 55 66 77 88", 10),
        ("F0 0F C1 05 11 22 33 44", 8),
        ("C7 05 11 22 33 44 01 00 00 00", 10),
        ("F6 05 11 22 33 44 01", 7),
        ("F7 D8", 2),
        ("66 0F C5 C0 01", 5),
        ("48 63 C8", 3),
        ("48 69 C0 11 22 33 44", 7),
        ("48 6B C8 58", 4),
        ("66 C7 00 01 00", 5),
        ("41 FF D0", 3),
        ("4C 8D 34 F0", 4),
        ("0F B6 04 08", 4),
    ] {
        assert_eq!(length(hex, true), len, "{hex}");
    }
}

#[test]
fn measures_x86_instructions() {
    for (hex, len) in [
        ("A1 11 22 33 44", 5),
        ("8B 0D 11 22 33 44", 6),
        ("55", 1),
        ("B8 11 22 33 44", 5),
        ("66 B8 11 22", 4),
        ("64 A1 00 00 00 00", 6),
        ("68 11 22 33 44", 5),
        ("6A 10", 2),
        ("69 C0 11 22 33 44", 6),
        ("40", 1),
        ("C1 EA 06", 3),
        ("83 3C B0 00", 4),
        ("8B 34 B8", 3),
    ] {
        assert_eq!(length(hex, false), len, "{hex}");
    }
}

#[test]
fn tells_where_control_goes() {
    assert_eq!(flow("48 8B 05 11 22 33 44", true), Flow::Next);
    assert_eq!(flow("E8 11 22 33 44", true), Flow::Call(0x4433_2211));
    assert_eq!(flow("E9 11 22 33 44", true), Flow::Jump(0x4433_2211));
    assert_eq!(flow("EB F0", true), Flow::Jump(-0x10));
    assert_eq!(flow("74 10", true), Flow::Branch(0x10));
    assert_eq!(flow("0F 85 11 22 33 44", true), Flow::Branch(0x4433_2211));
    assert_eq!(flow("C3", true), Flow::Stop);
    assert_eq!(flow("C2 08 00", false), Flow::Stop);
    assert_eq!(flow("CC", true), Flow::Stop);
    assert_eq!(flow("0F 0B", false), Flow::Stop);
    // An indirect jump leaves the function to somewhere the code doesn't say.
    assert_eq!(flow("FF 25 11 22 33 44", true), Flow::Stop);
    assert_eq!(flow("FF 24 85 11 22 33 44", false), Flow::Stop);
    // An indirect call returns, so the function goes on after it.
    assert_eq!(flow("FF 15 11 22 33 44", true), Flow::Next);
    assert_eq!(flow("41 FF D0", true), Flow::Next);
    // A two-byte opcode whose displacement holds a byte like a branch's
    // second opcode byte is still no branch.
    assert_eq!(flow("0F B6 80 00 00 85 00", true), Flow::Next);
}

#[test]
fn refuses_instructions_cut_off_at_the_end() {
    assert!(decode(&bytes("48 8B 05 11 22"), true).is_none());
    assert!(decode(&bytes("E8 11"), true).is_none());
    assert!(decode(&[], true).is_none());
}

#[test]
fn measures_unusual_prefix_orders_and_operand_sizes() {
    for (hex, len) in [
        // A REX byte only counts right before the opcode.
        ("48 66 90", 3),
        ("40 40 90", 3),
        // REX.W wins over the 66 prefix for the immediate's size.
        ("66 48 C7 C0 01 00 00 00", 8),
        ("66 48 81 C0 01 00 00 00", 8),
        // 3DNow ends with an 8-bit immediate.
        ("0F 0F C1 B4", 4),
    ] {
        assert_eq!(length(hex, true), len, "{hex}");
    }
    for (hex, len) in [("D4 0A", 2), ("D5 0A", 2)] {
        assert_eq!(length(hex, false), len, "{hex}");
    }
    // x64 ignores the 66 prefix on near calls and jumps.
    assert_eq!(flow("66 E8 11 22 33 44", true), Flow::Call(0x4433_2211));
    assert_eq!(length("66 E9 11 22 33 44", true), 6);
}

#[test]
fn refuses_vex_and_evex() {
    assert!(decode(&bytes("62 F1 7C 48 10 00"), true).is_none());
    assert!(decode(&bytes("C5 F8 77"), false).is_none());
    assert!(decode(&bytes("C4 E2 79 18 00"), false).is_none());
    // LES and LDS with a memory operand are still decoded on x86.
    assert_eq!(length("C5 06", false), 2);
}

#[test]
fn refuses_an_unsupported_16_bit_near_conditional_branch() {
    assert!(decode(&bytes("66 0F 85 11 22 90 90"), false).is_none());
}

#[test]
fn refuses_evex_in_32_bit_mode_too() {
    assert!(decode(&bytes("62 F1 7C 48 10 00"), false).is_none());
}
