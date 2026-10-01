//! Finds the globals the walk starts from through functions IL2CPP exports
//! by name. Exports are always there, so the search starts from code that
//! does the same job in every build.

use arrayvec::ArrayVec;

use crate::{file_format::pe, signature::Signature, Address, PointerSize, Process};

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

/// Reads the operand at `at` and returns the global it points to. A 64-bit
/// build gives the global as a displacement from the end of the operand, a
/// 32-bit build as the address itself.
fn operand(process: &Process, at: Address, pointer_size: PointerSize) -> Option<Address> {
    match pointer_size {
        PointerSize::Bit64 => Some(at + 4 + process.read::<i32>(at).ok()?),
        _ => Some(Address::new(process.read::<u32>(at).ok()? as u64)),
    }
}

/// Finds `s_Assemblies` through `il2cpp_domain_get_assemblies`. A release
/// build calls a getter that returns its address. A master build loads it
/// right in the export. The getter goes first, because the master shape can
/// also show up in the function after a short release export.
pub(super) fn assemblies(
    process: &Process,
    module: (Address, u64),
    pointer_size: PointerSize,
) -> Option<Address> {
    let export = export(process, module, "il2cpp_domain_get_assemblies")?;
    getter(process, export, pointer_size).or_else(|| master(process, export, pointer_size))
}

/// Finds `s_Assemblies` through the first call in the export that goes to
/// the getter.
fn getter(process: &Process, export: Address, pointer_size: PointerSize) -> Option<Address> {
    // lea rax, [s_Assemblies]; ret
    const GETTER_X64: Signature<8> = Signature::new("48 8D 05 ?? ?? ?? ?? C3");
    // mov eax, s_Assemblies; ret
    const GETTER_X86: Signature<6> = Signature::new("B8 ?? ?? ?? ?? C3");
    let code = process.read::<[u8; 0x20]>(export).ok()?;
    (0..code.len() - 4)
        .filter(|&at| code[at] == 0xE8)
        .find_map(|at| {
            let rel = i32::from_le_bytes(code[at + 1..at + 5].try_into().ok()?);
            let getter = export + (at as u64 + 5) + rel;
            let at = match pointer_size {
                PointerSize::Bit64 => {
                    GETTER_X64
                        .scan_process_range(process, (getter, 8))
                        .filter(|&found| found == getter)?
                        + 3
                }
                _ => {
                    GETTER_X86
                        .scan_process_range(process, (getter, 6))
                        .filter(|&found| found == getter)?
                        + 1
                }
            };
            operand(process, at, pointer_size)
        })
}

/// Finds `s_Assemblies` where the export loads it and returns.
fn master(process: &Process, export: Address, pointer_size: PointerSize) -> Option<Address> {
    // mov rax, [s_Assemblies]; ret
    const MASTER_X64: Signature<8> = Signature::new("48 8B 05 ?? ?? ?? ?? C3");
    // mov eax, [s_Assemblies]; pop ebp; ret
    const MASTER_X86: Signature<7> = Signature::new("A1 ?? ?? ?? ?? 5D C3");
    let at = match pointer_size {
        PointerSize::Bit64 => MASTER_X64.scan_process_range(process, (export, 0x40))? + 3,
        _ => MASTER_X86.scan_process_range(process, (export, 0x40))? + 1,
    };
    operand(process, at, pointer_size)
}

/// How many bytes of each function the table search reads.
const WINDOW: usize = 0xC0;
/// How many calls and jumps deep the table search follows.
const DEPTH: usize = 4;
/// How many functions one level of the table search holds. The widest level
/// on the test players holds 140. Functions past this many aren't searched.
const LEVEL: usize = 512;
/// How many functions the table search reads in all. Each one is read once.
const SEEN: usize = 1024;

/// Finds `s_TypeInfoDefinitionTable` through `il2cpp_image_get_class`, or
/// through `il2cpp_class_from_type` where the first one isn't exported.
/// Either reaches the type accessor within a few calls and jumps. The
/// accessor loads the table and reads it at the type index, which tells it
/// apart from the other globals on the way.
pub(super) fn type_info_definition_table(
    process: &Process,
    module: (Address, u64),
    pointer_size: PointerSize,
) -> Option<Address> {
    let start = export(process, module, "il2cpp_image_get_class")
        .or_else(|| export(process, module, "il2cpp_class_from_type"))?;
    let end = module.0 + module.1;

    let mut seen = ArrayVec::<Address, SEEN>::new();
    let mut level = ArrayVec::<Address, LEVEL>::new();
    level.push(start);
    for _ in 0..=DEPTH {
        let mut next = ArrayVec::<Address, LEVEL>::new();
        for &function in &level {
            if seen.contains(&function) || seen.try_push(function).is_err() {
                continue;
            }
            // The window stops at the module's end.
            let len = WINDOW.min(end.value().saturating_sub(function.value()) as usize);
            if let Some(table) = indexed_load(process, (function, len as u64), pointer_size) {
                return Some(table);
            }
            let mut code = [0; WINDOW];
            if process.read_into_slice(function, &mut code[..len]).is_err() {
                continue;
            }
            // Every call and jump, E8 and E9 with a 32-bit displacement.
            for at in 0..len.saturating_sub(4) {
                if code[at] != 0xE8 && code[at] != 0xE9 {
                    continue;
                }
                let rel =
                    i32::from_le_bytes([code[at + 1], code[at + 2], code[at + 3], code[at + 4]]);
                let target = function + (at as u64 + 5) + rel;
                if target >= module.0 && target < end && !next.contains(&target) {
                    let _ = next.try_push(target);
                }
            }
        }
        level = next;
    }
    None
}

/// Finds the first load of a global in the window that is read at an index
/// right after.
fn indexed_load(
    process: &Process,
    window: (Address, u64),
    pointer_size: PointerSize,
) -> Option<Address> {
    // mov rax, [table]; cmp qword ptr [reg + rax], 0
    const COMPARED_X64: Signature<12> = Signature::new("48 8B 05 ?? ?? ?? ?? 48 83 3C ?? 00");
    // mov rax, [table]; lea rsi or r14, [rax + reg * 8]
    const ADDRESSED_X64: Signature<11> = Signature::new("48 8B 05 ?? ?? ?? ?? ?? 8D 34 ??");
    // mov eax, [table]; cmp dword ptr [eax + reg * 4], 0
    const COMPARED_X86: Signature<9> = Signature::new("A1 ?? ?? ?? ?? 83 3C ?? 00");
    // mov eax, [table]; mov esi, dword ptr [eax + reg * 4]
    const LOADED_X86: Signature<8> = Signature::new("A1 ?? ?? ?? ?? 8B 34 ??");

    let (load, operand_at) = match pointer_size {
        PointerSize::Bit64 => (
            [
                COMPARED_X64.scan_process_range(process, window),
                ADDRESSED_X64.scan_process_range(process, window),
            ],
            3,
        ),
        _ => (
            [
                COMPARED_X86.scan_process_range(process, window),
                LOADED_X86.scan_process_range(process, window),
            ],
            1,
        ),
    };
    let load = load.into_iter().flatten().min()?;
    operand(process, load + operand_at, pointer_size)
}
