//! Finds the globals the walk starts from through functions IL2CPP exports
//! by name. Exports are always there, so the search starts from code that
//! does the same job in every build. From there it follows the code one
//! real instruction at a time, so it never reads into a neighboring
//! function or takes bytes inside an operand for a call.

use arrayvec::ArrayVec;

use super::instruction::{decode, Flow};
use crate::{file_format::pe, Address, PointerSize, Process};

/// Finds the address of a function the module exports by this name.
fn export(process: &Process, module: (Address, u64), name: &str) -> Option<Address> {
    pe::symbols(process, module.0)
        .find(|symbol| {
            symbol
                .get_name::<48>(process)
                .is_ok_and(|symbol_name| symbol_name.matches(name))
        })
        .map(|symbol| symbol.address)
}

/// Finds `s_Assemblies` through `il2cpp_domain_get_assemblies`. A release
/// build calls a getter that returns its address. A master build loads it
/// right in the export.
pub(super) fn assemblies(
    process: &Process,
    module: (Address, u64),
    pointer_size: PointerSize,
) -> Option<Address> {
    let start = export(process, module, "il2cpp_domain_get_assemblies")?;
    walk(process, module, start, pointer_size, 1, |code| {
        if pointer_size == PointerSize::Bit64 {
            // lea rax, [s_Assemblies]; ret, or mov rax, [s_Assemblies]; ret
            let load =
                bytes_at(code, 0, &[0x48, 0x8D, 0x05]) || bytes_at(code, 0, &[0x48, 0x8B, 0x05]);
            (load && bytes_at(code, 7, &[0xC3])).then_some(3)
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
    pointer_size: PointerSize,
) -> Option<Address> {
    let start = export(process, module, "il2cpp_image_get_class")
        .or_else(|| export(process, module, "il2cpp_type_get_class_or_element_class"))?;
    walk(process, module, start, pointer_size, 4, |code| {
        if pointer_size == PointerSize::Bit64 {
            // mov rax, [table]; then cmp qword ptr [reg + rax], 0, or
            // lea rsi or r14, [rax + reg * 8]
            let compared = bytes_at(code, 7, &[0x48, 0x83, 0x3C]) && bytes_at(code, 11, &[0]);
            let addressed = bytes_at(code, 8, &[0x8D, 0x34]);
            (bytes_at(code, 0, &[0x48, 0x8B, 0x05]) && (compared || addressed)).then_some(3)
        } else {
            // mov eax, [table]; then cmp dword ptr [eax + reg * 4], 0, or
            // mov esi, dword ptr [eax + reg * 4]
            let compared = bytes_at(code, 5, &[0x83, 0x3C]) && bytes_at(code, 8, &[0]);
            let loaded = bytes_at(code, 5, &[0x8B, 0x34]);
            (bytes_at(code, 0, &[0xA1]) && (compared || loaded)).then_some(1)
        }
    })
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
            let mut branches = ArrayVec::<Address, BRANCHES>::new();
            let mut started = ArrayVec::<Address, BRANCHES>::new();
            branches.push(function);
            let mut steps = 0;
            while let Some(mut at) = branches.pop() {
                if started.contains(&at) || started.try_push(at).is_err() {
                    continue;
                }
                while steps < STEPS {
                    steps += 1;
                    let Some(bytes) = code.at(at) else { break };
                    let Some(instruction) = decode(bytes, x64) else {
                        break;
                    };
                    if let Some(offset) = look(bytes) {
                        let operand = &bytes[offset..offset + 4];
                        let value =
                            u32::from_le_bytes([operand[0], operand[1], operand[2], operand[3]]);
                        return Some(if x64 {
                            at + offset as u64 + 4 + value as i32
                        } else {
                            Address::new(value as u64)
                        });
                    }
                    let after = at + instruction.len as u64;
                    let target = |rel: i32| Some(after + rel).filter(|&target| inside(target));
                    match instruction.flow {
                        Flow::Next => {}
                        Flow::Branch(rel) => {
                            if let Some(target) = target(rel) {
                                let _ = branches.try_push(target);
                            }
                        }
                        Flow::Call(rel) => {
                            if let Some(target) = target(rel) {
                                let _ = next.try_push(target);
                            }
                        }
                        Flow::Jump(rel) => {
                            if let Some(target) = target(rel) {
                                let _ = next.try_push(target);
                            }
                            break;
                        }
                        Flow::Stop => break,
                    }
                    at = after;
                }
            }
        }
        level = next;
    }
    None
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
    fn new(process: &'a Process, end: Address) -> Self {
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
