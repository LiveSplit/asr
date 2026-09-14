//! Support for parsing Mach-O format

use core::{fmt, mem};

#[cfg(feature = "alloc")]
use core::iter::FusedIterator;

#[cfg(feature = "alloc")]
use alloc::collections::BTreeMap;

use bytemuck::{Pod, Zeroable};

#[cfg(feature = "alloc")]
use crate::{string::ArrayCString, Error};
use crate::{Address, PointerSize, Process};

// Magic mach-o header constants from:
// https://opensource.apple.com/source/xnu/xnu-4570.71.2/EXTERNAL_HEADERS/mach-o/loader.h.auto.html
const MH_MAGIC_32: u32 = 0xfeedface;
const MH_CIGAM_32: u32 = 0xcefaedfe;
const MH_MAGIC_64: u32 = 0xfeedfacf;
const MH_CIGAM_64: u32 = 0xcffaedfe;

/// Checks if a given Mach-O module is 64-bit or 32-bit
pub fn pointer_size(process: &Process, range: (Address, u64)) -> Option<PointerSize> {
    match process.read::<u32>(scan_macho_page(process, range)?).ok()? {
        MH_MAGIC_64 | MH_CIGAM_64 => Some(PointerSize::Bit64),
        MH_MAGIC_32 | MH_CIGAM_32 => Some(PointerSize::Bit32),
        _ => None,
    }
}

/// Scans the range for a page that begins with Mach-O Magic
fn scan_macho_page(process: &Process, range: (Address, u64)) -> Option<Address> {
    const PAGE_SIZE: u64 = 0x1000;
    let (addr, len) = range;
    // negation mod PAGE_SIZE
    let distance_to_page = (PAGE_SIZE - (addr.value() % PAGE_SIZE)) % PAGE_SIZE;
    // round up to the next multiple of PAGE_SIZE
    let first_page = addr + distance_to_page;
    for i in 0..((len - distance_to_page) / PAGE_SIZE) {
        let a = first_page + (i * PAGE_SIZE);
        if let Ok(MH_MAGIC_64 | MH_CIGAM_64 | MH_MAGIC_32 | MH_CIGAM_32) = process.read::<u32>(a) {
            return Some(a);
        }
    }
    None
}

/// Scans the range for pages that begin with Mach-O Magic
#[cfg(feature = "alloc")]
fn scan_macho_pages(
    process: &Process,
    range: (Address, u64),
) -> impl FusedIterator<Item = Address> + '_ {
    const PAGE_SIZE: u64 = 0x1000;
    let (addr, len) = range;
    // negation mod PAGE_SIZE
    let distance_to_page = (PAGE_SIZE - (addr.value() % PAGE_SIZE)) % PAGE_SIZE;
    // round up to the next multiple of PAGE_SIZE
    let first_page = addr + distance_to_page;
    (0..((len - distance_to_page) / PAGE_SIZE))
        .filter_map(move |i| {
            let a = first_page + (i * PAGE_SIZE);
            match process.read::<u32>(a) {
                Ok(MH_MAGIC_64 | MH_CIGAM_64 | MH_MAGIC_32 | MH_CIGAM_32) => Some(a),
                _ => None,
            }
        })
        .fuse()
}

// Constants for the cmd field of load commands, the type
// https://opensource.apple.com/source/xnu/xnu-4570.71.2/EXTERNAL_HEADERS/mach-o/loader.h.auto.html
/// the uuid
const LC_UUID: u32 = 0x1b;
/// link-edit stab symbol table info
#[cfg(feature = "alloc")]
const LC_SYMTAB: u32 = 0x2;
/// 64-bit segment of this file to be mapped
#[cfg(feature = "alloc")]
const LC_SEGMENT_64: u32 = 0x19;

/// The UUID of a Mach-O module, from its `LC_UUID` load command. The linker
/// derives it from the built binary, so it names one exact build.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Uuid {
    /// The bytes of the UUID.
    pub bytes: [u8; 16],
}

impl fmt::Debug for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, byte) in self.bytes.iter().enumerate() {
            if let 4 | 6 | 8 | 10 = i {
                f.write_str("-")?;
            }
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Reads the UUID from the load commands of the Mach-O module in the given
/// range. Returns [`None`] if the module carries no `LC_UUID` command, or is
/// 32-bit or big-endian. macOS has not run 32-bit software since 2019, and
/// Unity dropped PowerPC, the only big-endian Mac, with Unity 3.
pub fn uuid(process: &Process, range: (Address, u64)) -> Option<Uuid> {
    #[derive(Debug, Copy, Clone, Zeroable, Pod)]
    #[repr(C)]
    struct MachHeader {
        magic: u32,
        cputype: u32,
        cpusubtype: u32,
        filetype: u32,
        ncmds: u32,
        sizeofcmds: u32,
        flags: u32,
    }

    #[derive(Debug, Copy, Clone, Zeroable, Pod)]
    #[repr(C)]
    struct LoadCommand {
        cmd: u32,
        cmdsize: u32,
    }

    let page = scan_macho_page(process, range)?;
    let header = process.read::<MachHeader>(page).ok()?;

    // Anything but the 64-bit little-endian magic is 32-bit or big-endian,
    // and nothing here decodes either. The 64-bit header ends with one
    // reserved field.
    if header.magic != MH_MAGIC_64 {
        return None;
    }
    let commands = page + (mem::size_of::<MachHeader>() + mem::size_of::<u32>()) as u64;

    // The command table has to lie inside the module, which is what bounds
    // the walk.
    let (module_address, module_size) = range;
    let table_size = header.sizeofcmds as u64;
    let module_end = module_address.value().saturating_add(module_size);
    if commands
        .value()
        .checked_add(table_size)
        .is_none_or(|end| end > module_end)
    {
        return None;
    }

    let mut offset = 0;
    for _ in 0..header.ncmds {
        if offset + mem::size_of::<LoadCommand>() as u64 > table_size {
            return None;
        }
        let command = process.read::<LoadCommand>(commands + offset).ok()?;
        let size = command.cmdsize as u64;
        if size < mem::size_of::<LoadCommand>() as u64 || offset + size > table_size {
            return None;
        }

        if command.cmd == LC_UUID {
            if size < (mem::size_of::<LoadCommand>() + mem::size_of::<Uuid>()) as u64 {
                return None;
            }
            return process
                .read::<[u8; 16]>(commands + offset + mem::size_of::<LoadCommand>() as u64)
                .ok()
                .map(|bytes| Uuid { bytes });
        }

        offset += size;
    }

    None
}

#[cfg(feature = "alloc")]
struct MachOFormatOffsets {
    number_of_commands: u32,
    load_commands: u32,
    command_size: u32,
    symtab_offset: u32,
    number_of_symbols: u32,
    strtab_offset: u32,
    nlist_value: u32,
    size_of_nlist_item: u32,
    segcmd64_vmaddr: u32,
    segcmd64_fileoff: u32,
}

#[cfg(feature = "alloc")]
impl MachOFormatOffsets {
    const fn new() -> Self {
        // offsets taken from:
        //  - https://github.com/hackf5/unityspy/blob/master/src/HackF5.UnitySpy/Offsets/MachOFormatOffsets.cs
        //  - https://opensource.apple.com/source/xnu/xnu-4570.71.2/EXTERNAL_HEADERS/mach-o/loader.h.auto.html
        MachOFormatOffsets {
            number_of_commands: 0x10,
            load_commands: 0x20,
            command_size: 0x04,
            symtab_offset: 0x08,
            number_of_symbols: 0x0c,
            strtab_offset: 0x10,
            nlist_value: 0x08,
            size_of_nlist_item: 0x10,
            segcmd64_vmaddr: 0x18,
            segcmd64_fileoff: 0x28,
        }
    }
}

/// A symbol exported into the current module.
#[cfg(feature = "alloc")]
pub struct Symbol {
    /// The address associated with the current function
    pub address: Address,
    /// The address storing the name of the current function
    name_addr: Address,
}

#[cfg(feature = "alloc")]
impl Symbol {
    /// Tries to retrieve the name of the current function
    pub fn get_name<const CAP: usize>(
        &self,
        process: &Process,
    ) -> Result<ArrayCString<CAP>, Error> {
        process.read(self.name_addr)
    }
}

/// Iterates over the exported symbols for a given module.
/// Only 64-bit Mach-O format is supported
#[cfg(feature = "alloc")]
pub fn symbols(process: &Process, range: (Address, u64)) -> impl FusedIterator<Item = Symbol> + '_ {
    scan_macho_pages(process, range)
        .filter_map(|page| macho_page_symbols(process, page))
        .flatten()
        .fuse()
}

#[cfg(feature = "alloc")]
fn macho_page_symbols(
    process: &Process,
    page: Address,
) -> Option<impl FusedIterator<Item = Symbol> + '_> {
    let offsets = MachOFormatOffsets::new();
    let number_of_commands: u32 = process.read(page + offsets.number_of_commands).ok()?;

    let mut symtab_fileoff: u32 = 0;
    let mut number_of_symbols: u32 = 0;
    let mut strtab_fileoff: u32 = 0;
    let mut map_fileoff_to_vmaddr: BTreeMap<u64, u64> = BTreeMap::new();

    let mut next: u32 = offsets.load_commands;
    for _i in 0..number_of_commands {
        let cmdtype: u32 = process.read(page + next).ok()?;
        if cmdtype == LC_SYMTAB {
            symtab_fileoff = process.read(page + next + offsets.symtab_offset).ok()?;
            number_of_symbols = process.read(page + next + offsets.number_of_symbols).ok()?;
            strtab_fileoff = process.read(page + next + offsets.strtab_offset).ok()?;
        } else if cmdtype == LC_SEGMENT_64 {
            let vmaddr: u64 = process.read(page + next + offsets.segcmd64_vmaddr).ok()?;
            let fileoff: u64 = process.read(page + next + offsets.segcmd64_fileoff).ok()?;
            map_fileoff_to_vmaddr.insert(fileoff, vmaddr);
        }
        let command_size: u32 = process.read(page + next + offsets.command_size).ok()?;
        next += command_size;
    }

    if symtab_fileoff == 0 || number_of_symbols == 0 || strtab_fileoff == 0 {
        return None;
    }

    let symtab_vmaddr = fileoff_to_vmaddr(&map_fileoff_to_vmaddr, symtab_fileoff as u64);
    let strtab_vmaddr = fileoff_to_vmaddr(&map_fileoff_to_vmaddr, strtab_fileoff as u64);

    Some(
        (0..number_of_symbols)
            .filter_map(move |j| {
                let nlist_item = page + symtab_vmaddr + (j * offsets.size_of_nlist_item);
                let symname_offset: u32 = process.read(nlist_item).ok()?;
                let string_address = page + strtab_vmaddr + symname_offset;
                let symbol_fileoff = process.read(nlist_item + offsets.nlist_value).ok()?;
                let symbol_vmaddr = fileoff_to_vmaddr(&map_fileoff_to_vmaddr, symbol_fileoff);
                let symbol_address = page + symbol_vmaddr;
                Some(Symbol {
                    address: symbol_address,
                    name_addr: string_address,
                })
            })
            .fuse(),
    )
}

#[cfg(feature = "alloc")]
fn fileoff_to_vmaddr(map: &BTreeMap<u64, u64>, fileoff: u64) -> u64 {
    map.iter()
        .filter(|(&k, _)| k <= fileoff)
        .max_by_key(|(&k, _)| k)
        .map(|(&k, &v)| v + fileoff - k)
        .unwrap_or(fileoff)
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::uuid;
    use crate::runtime::mock::with_process;

    use std::{format, vec, vec::Vec};

    const BASE: u64 = 0x1_0000_0000;

    // The UUID of the 2019.4 mono runtime shipped with a real mac player.
    const UUID: [u8; 16] = [
        0xE7, 0x42, 0x0B, 0xC7, 0xA2, 0x6B, 0x33, 0xFA, 0xB5, 0xCD, 0x41, 0xCA, 0xD7, 0xD4, 0x61,
        0x4C,
    ];

    fn put(image: &mut [u8], at: usize, bytes: &[u8]) {
        image[at..at + bytes.len()].copy_from_slice(bytes);
    }

    // Builds a minimal mapped Mach-O by hand from the loader header, so the
    // walk is checked against the format rather than against itself: the
    // header declaring its command table, a segment command, and the uuid
    // command.
    fn image() -> Vec<u8> {
        let mut image = vec![0; 0x1000];
        put(&mut image, 0x00, &0xFEEDFACF_u32.to_le_bytes());
        put(&mut image, 0x10, &2_u32.to_le_bytes());
        put(&mut image, 0x14, &(0x48 + 24_u32).to_le_bytes());
        let commands = 0x20;
        put(&mut image, commands, &0x19_u32.to_le_bytes());
        put(&mut image, commands + 0x4, &0x48_u32.to_le_bytes());
        put(&mut image, commands + 0x48, &0x1B_u32.to_le_bytes());
        put(&mut image, commands + 0x4C, &24_u32.to_le_bytes());
        put(&mut image, commands + 0x50, &UUID);
        image
    }

    #[test]
    fn reads_the_uuid_from_a_mapped_image() {
        with_process(&[(BASE, &image())], |process| {
            let uuid = uuid(process, (BASE.into(), 0x1000)).unwrap();
            assert_eq!(uuid.bytes, UUID);
        });
    }

    #[test]
    fn answers_nothing_for_a_32_bit_image() {
        let mut image = image();
        put(&mut image, 0x00, &0xFEEDFACE_u32.to_le_bytes());
        with_process(&[(BASE, &image)], |process| {
            assert!(uuid(process, (BASE.into(), 0x1000)).is_none());
        });
    }

    #[test]
    fn renders_the_uuid_canonically() {
        with_process(&[(BASE, &image())], |process| {
            let uuid = uuid(process, (BASE.into(), 0x1000)).unwrap();
            assert_eq!(format!("{uuid:?}"), "e7420bc7-a26b-33fa-b5cd-41cad7d4614c");
        });
    }

    #[test]
    fn answers_nothing_without_a_uuid_command() {
        let mut image = image();
        put(&mut image, 0x20 + 0x48, &0_u32.to_le_bytes());
        with_process(&[(BASE, &image)], |process| {
            assert!(uuid(process, (BASE.into(), 0x1000)).is_none());
        });
    }

    #[test]
    fn answers_nothing_when_the_commands_overrun_the_module() {
        let mut image = image();
        put(&mut image, 0x14, &0x2000_u32.to_le_bytes());
        with_process(&[(BASE, &image)], |process| {
            assert!(uuid(process, (BASE.into(), 0x1000)).is_none());
        });
    }

    #[test]
    fn reads_the_uuid_past_the_sixty_fourth_command() {
        let mut image = vec![0; 0x1000];
        put(&mut image, 0x00, &0xFEEDFACF_u32.to_le_bytes());
        put(&mut image, 0x10, &66_u32.to_le_bytes());
        put(&mut image, 0x14, &(65 * 8 + 24_u32).to_le_bytes());
        // Sixty-five empty commands ahead of the uuid one.
        let mut at = 0x20;
        for _ in 0..65 {
            put(&mut image, at + 4, &8_u32.to_le_bytes());
            at += 8;
        }
        put(&mut image, at, &0x1B_u32.to_le_bytes());
        put(&mut image, at + 4, &24_u32.to_le_bytes());
        put(&mut image, at + 8, &UUID);
        with_process(&[(BASE, &image)], |process| {
            let uuid = uuid(process, (BASE.into(), 0x1000)).unwrap();
            assert_eq!(uuid.bytes, UUID);
        });
    }

    #[test]
    fn answers_nothing_for_a_big_endian_image() {
        let mut image = image();
        put(&mut image, 0x00, &0xFEEDFACF_u32.to_be_bytes());
        with_process(&[(BASE, &image)], |process| {
            assert!(uuid(process, (BASE.into(), 0x1000)).is_none());
        });
    }
}
