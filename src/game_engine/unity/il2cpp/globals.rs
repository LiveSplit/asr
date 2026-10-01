//! Finds the globals the walk starts from through functions IL2CPP exports
//! by name. Exports are always there, so the search starts from code that
//! does the same job in every build.

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
