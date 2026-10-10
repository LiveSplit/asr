//! Finds the globals the walk starts from through functions IL2CPP exports
//! by name. Exports are always there, so the search starts from code that
//! does the same job in every build. From there it follows the code one
//! real instruction at a time, so it never reads into a neighboring
//! function or takes bytes inside an operand for a call.

use arrayvec::ArrayVec;

use super::{
    super::BinaryFormat,
    instruction::{decode, Flow},
};
use crate::{
    file_format::{elf, pe},
    string::ArrayCString,
    Address, PointerSize, Process,
};

/// Finds the address of a function the module exports by this name.
fn export(
    process: &Process,
    module: (Address, u64),
    format: BinaryFormat,
    name: &str,
) -> Option<Address> {
    let matches = |symbol_name: ArrayCString<48>| symbol_name.matches(name);
    match format {
        BinaryFormat::PE => pe::symbols(process, module.0)
            .find(|symbol| symbol.get_name::<48>(process).is_ok_and(matches))
            .map(|symbol| symbol.address),
        _ => elf::symbols(process, module.0)
            .find(|symbol| symbol.get_name::<48>(process).is_ok_and(matches))
            .map(|symbol| symbol.address),
    }
}

/// Finds `s_Assemblies` through `il2cpp_domain_get_assemblies`. A release
/// build calls a getter that returns its address. A master build loads it
/// right in the export.
pub(super) fn assemblies(
    process: &Process,
    module: (Address, u64),
    format: BinaryFormat,
    pointer_size: PointerSize,
) -> Option<Address> {
    let start = export(process, module, format, "il2cpp_domain_get_assemblies")?;
    let linux = format == BinaryFormat::ELF;
    walk(process, module, format, start, pointer_size, 1, |code| {
        if pointer_size == PointerSize::Bit64 {
            // lea rax, [s_Assemblies]; ret, or mov rax, [s_Assemblies]; ret.
            // On Linux a getter can pop between the two to keep its stack
            // aligned, and the master export can go on with sub rcx, rax.
            let lea = bytes_at(code, 0, &[0x48, 0x8D, 0x05]);
            let mov = bytes_at(code, 0, &[0x48, 0x8B, 0x05]);
            let popped =
                linux && matches!(code.get(7), Some(0x58..=0x5F)) && bytes_at(code, 8, &[0xC3]);
            let returned = (lea || mov) && (bytes_at(code, 7, &[0xC3]) || popped);
            let subtracted = linux && mov && bytes_at(code, 7, &[0x48, 0x29, 0xC1]);
            (returned || subtracted).then_some(3)
        } else {
            // mov eax, s_Assemblies; ret, or mov eax, [s_Assemblies]; pop ebp; ret
            let getter = bytes_at(code, 0, &[0xB8]) && bytes_at(code, 5, &[0xC3]);
            let master = bytes_at(code, 0, &[0xA1]) && bytes_at(code, 5, &[0x5D, 0xC3]);
            (getter || master).then_some(1)
        }
    })
}

/// Finds `s_TypeInfoDefinitionTable` through `il2cpp_image_get_class`, or
/// through `il2cpp_type_get_class_or_element_class` where the first one isn't
/// exported. Either reaches the type accessor within a few calls and jumps.
/// The accessor loads the table and reads it at the type index, which tells
/// it apart from the other globals on the way.
pub(super) fn type_info_definition_table(
    process: &Process,
    module: (Address, u64),
    format: BinaryFormat,
    pointer_size: PointerSize,
) -> Option<Address> {
    let start = export(process, module, format, "il2cpp_image_get_class").or_else(|| {
        export(
            process,
            module,
            format,
            "il2cpp_type_get_class_or_element_class",
        )
    })?;
    let linux = format == BinaryFormat::ELF;
    walk(process, module, format, start, pointer_size, 4, |code| {
        if pointer_size == PointerSize::Bit64 {
            // mov rax, [table]; then cmp qword ptr [reg + rax], 0, or
            // lea rsi or r14, [rax + reg * 8]
            let compared = bytes_at(code, 7, &[0x48, 0x83, 0x3C]) && bytes_at(code, 11, &[0]);
            let addressed =
                matches!(code.get(7), Some(0x48 | 0x4C)) && bytes_at(code, 8, &[0x8D, 0x34]);
            let windows = bytes_at(code, 0, &[0x48, 0x8B, 0x05]) && (compared || addressed);
            (windows || (linux && indexed_load(code))).then_some(3)
        } else {
            // mov eax, [table]; then cmp dword ptr [eax + reg * 4], 0, or
            // mov esi, dword ptr [eax + reg * 4]
            let compared = bytes_at(code, 5, &[0x83, 0x3C]) && bytes_at(code, 8, &[0]);
            let loaded = bytes_at(code, 5, &[0x8B, 0x34]);
            (bytes_at(code, 0, &[0xA1]) && (compared || loaded)).then_some(1)
        }
    })
}

/// Checks whether `code` starts with a load of a global into a register that
/// one of the next 2 instructions reads at an index times 8, the way clang
/// compiles the type accessor on Linux. A return, jump, call or branch ends
/// the search, since the code after it may never run.
fn indexed_load(code: &[u8]) -> bool {
    let (Some(&rex), Some(&0x8B), Some(&modrm)) = (code.first(), code.get(1), code.get(2)) else {
        return false;
    };
    if !matches!(rex, 0x48 | 0x4C) || modrm & 0xC7 != 0x05 {
        return false;
    }
    let register = ((rex & 0x04) << 1) | ((modrm >> 3) & 7);
    let mut at = 7;
    for _ in 0..2 {
        let Some(next) = code.get(at..).and_then(|rest| decode(rest, true)) else {
            return false;
        };
        if next.flow != Flow::Next {
            return false;
        }
        if reads_scaled(&code[at..at + next.len], register) {
            return true;
        }
        at += next.len;
    }
    false
}

/// Checks whether an instruction reads memory at `[register + index * 8]`. A
/// base of rbp or r13 with mod 00 means a displacement and no base register.
fn reads_scaled(instruction: &[u8], register: u8) -> bool {
    let (rex, rest) = match instruction.first() {
        Some(&byte @ 0x40..=0x4F) => (byte, &instruction[1..]),
        _ => (0, instruction),
    };
    let rest = match rest.first() {
        Some(0x0F) => rest.get(1..).unwrap_or_default(),
        _ => rest,
    };
    let (Some(&modrm), Some(&sib)) = (rest.get(1), rest.get(2)) else {
        return false;
    };
    let base = ((rex & 0x01) << 3) | (sib & 7);
    let displacement = modrm >> 6 == 0 && sib & 7 == 5;
    modrm >> 6 != 3 && modrm & 7 == 4 && sib >> 6 == 3 && !displacement && base == register
}

/// Checks whether `code` holds `bytes` at `at`.
fn bytes_at(code: &[u8], at: usize, bytes: &[u8]) -> bool {
    code.get(at..at + bytes.len()) == Some(bytes)
}

/// How many functions one level of the walk holds. The widest level on the
/// test players holds 45.
const LEVEL: usize = 512;
/// How many functions the walk visits in all. A walk on the test players
/// visits at most 20.
const FUNCTIONS: usize = 1024;
/// How many places inside one function the walk can still have to go to.
const BRANCHES: usize = 64;
/// How many instructions the walk decodes in one function. The longest
/// function on the test players takes 334.
const STEPS: usize = 2048;

/// Walks the functions reachable from `start`, breadth first and at most
/// `depth` calls or jumps deep. Inside a function it follows every branch
/// and stops at a return or a jump. `look` gets the bytes at each
/// instruction and returns where an operand naming a global starts in them.
/// Returns the first global found that way.
fn walk(
    process: &Process,
    module: (Address, u64),
    format: BinaryFormat,
    start: Address,
    pointer_size: PointerSize,
    depth: usize,
    mut look: impl FnMut(&[u8]) -> Option<usize>,
) -> Option<Address> {
    let x64 = pointer_size == PointerSize::Bit64;
    let end = module.0 + module.1;
    let inside = |address: Address| address >= module.0 && address < end;
    let mut code = Code::new(process, end);
    let mut done = ArrayVec::<Address, FUNCTIONS>::new();
    let mut level = ArrayVec::<Address, LEVEL>::new();
    level.push(start);

    for _ in 0..=depth {
        let mut next = ArrayVec::<Address, LEVEL>::new();
        for &function in &level {
            if done.contains(&function) || done.try_push(function).is_err() {
                continue;
            }
            // An x64 PE image lists where each function starts and ends, and an
            // ELF image gives the size of each exported function, so the walk
            // stops at the end even after a call that never returns.
            let bounds = match format {
                BinaryFormat::PE if x64 => function_range(process, module.0, function),
                BinaryFormat::PE => None,
                _ => symbol_range(process, module.0, function),
            };
            let function_end = bounds.map_or(end, |(_, to)| to);
            let mut branches = ArrayVec::<Address, BRANCHES>::new();
            let mut walked = ArrayVec::<(Address, Address), BRANCHES>::new();
            branches.push(function);
            let mut steps = 0;
            while let Some(start) = branches.pop() {
                let mut at = start;
                while steps < STEPS
                    && at < function_end
                    && !walked.iter().any(|&(from, to)| at >= from && at < to)
                {
                    steps += 1;
                    let Some(bytes) = code.at(at) else { break };
                    // The bytes past the function's end belong to other code,
                    // so neither the decoder nor `look` gets to see them.
                    let left = (function_end.value() - at.value()) as usize;
                    let bytes = &bytes[..bytes.len().min(left)];
                    let Some(instruction) = decode(bytes, x64) else {
                        break;
                    };
                    // A match whose global lies outside the module is some
                    // other load, so the walk goes on past it.
                    let global = look(bytes).and_then(|offset| {
                        let operand = bytes.get(offset..offset + 4)?;
                        let value =
                            u32::from_le_bytes([operand[0], operand[1], operand[2], operand[3]]);
                        Some(if x64 {
                            at + offset as u64 + 4 + value as i32
                        } else {
                            Address::new(value as u64)
                        })
                    });
                    if let Some(global) = global.filter(|&global| inside(global)) {
                        return Some(global);
                    }
                    let after = at + instruction.len as u64;
                    let target = |rel: i32| Some(after + rel).filter(|&target| inside(target));
                    at = after;
                    match instruction.flow {
                        Flow::Next => {}
                        Flow::Branch(rel) => match target(rel) {
                            // A branch out of the function goes on in another
                            // one, like a jump, and this one goes on too.
                            Some(target)
                                if bounds
                                    .is_some_and(|(from, to)| target < from || target >= to) =>
                            {
                                if !next.contains(&target) {
                                    let _ = next.try_push(target);
                                }
                            }
                            Some(target) => {
                                let _ = branches.try_push(target);
                            }
                            None => {}
                        },
                        Flow::Call(rel) => {
                            if let Some(target) =
                                target(rel).filter(|target| !next.contains(target))
                            {
                                let _ = next.try_push(target);
                            }
                        }
                        Flow::Jump(rel) => {
                            // A jump inside the function goes on with it. A
                            // jump out of it goes on in another function.
                            let inner = |target: &Address| {
                                bounds.is_some_and(|(from, to)| *target >= from && *target < to)
                            };
                            match target(rel) {
                                Some(target) if inner(&target) => {
                                    let _ = branches.try_push(target);
                                }
                                Some(target) if !next.contains(&target) => {
                                    let _ = next.try_push(target);
                                }
                                _ => {}
                            }
                            break;
                        }
                        Flow::Stop => break,
                    }
                }
                if at > start {
                    let _ = walked.try_push((start, at));
                }
            }
        }
        level = next;
    }
    None
}

/// Finds the start and end of the x64 function that holds `address`, in the
/// exception directory of the module at `module`. Returns `None` when there
/// is none, as in x86 images.
fn function_range(
    process: &Process,
    module: Address,
    address: Address,
) -> Option<(Address, Address)> {
    // The exception directory is the fourth data directory of a PE32+ header,
    // and each of its entries holds the start, end and unwind info of one
    // function, sorted by start.
    let header = process.read::<u32>(module + 0x3C).ok()?;
    let [table, size] = process.read::<[u32; 2]>(module + header + 0xA0).ok()?;
    let offset = address.value().checked_sub(module.value())? as u32;
    let (mut low, mut high) = (0, size / 12);
    while low < high {
        let middle = (low + high) / 2;
        let [from, to, _] = process
            .read::<[u32; 3]>(module + table + middle * 12)
            .ok()?;
        if offset < from {
            high = middle;
        } else if offset >= to {
            low = middle + 1;
        } else {
            return Some((module + from, module + to));
        }
    }
    None
}

/// Finds the start and end of the exported function that holds `address`,
/// from the sizes the ELF dynamic symbols carry. Returns `None` for a function
/// nothing exports.
fn symbol_range(
    process: &Process,
    module: Address,
    address: Address,
) -> Option<(Address, Address)> {
    elf::symbols(process, module)
        .find(|symbol| {
            symbol.size != 0 && address >= symbol.address && address < symbol.address + symbol.size
        })
        .map(|symbol| (symbol.address, symbol.address + symbol.size))
}

/// The longest instruction is 15 bytes, and the patterns the walk looks for
/// reach 12 bytes past the start of one.
const AHEAD: usize = 16;

/// Code read from the process a block at a time, so the walk doesn't make a
/// call into the host for every instruction.
struct Code<'a> {
    process: &'a Process,
    end: Address,
    start: Address,
    len: usize,
    bytes: [u8; 256],
}

impl<'a> Code<'a> {
    const fn new(process: &'a Process, end: Address) -> Self {
        Self {
            process,
            end,
            start: Address::NULL,
            len: 0,
            bytes: [0; 256],
        }
    }

    /// Returns the bytes from `at` on, reading a new block when fewer than
    /// `AHEAD` of them are already read. Near the module's end there can be
    /// fewer.
    fn at(&mut self, at: Address) -> Option<&[u8]> {
        let left = self.end.value().checked_sub(at.value())? as usize;
        let offset = at.value().wrapping_sub(self.start.value()) as usize;
        if at < self.start || offset + AHEAD.min(left) > self.len {
            let len = self.bytes.len().min(left);
            self.process
                .read_into_slice(at, &mut self.bytes[..len])
                .ok()?;
            self.start = at;
            self.len = len;
            return Some(&self.bytes[..len]);
        }
        Some(&self.bytes[offset..self.len])
    }
}
