//! Support for parsing Windows Portable Executables.

use core::{fmt, mem};

use bytemuck::{Pod, Zeroable};

use crate::{string::ArrayCString, Address, Error, FromEndian, PointerSize, Process};

// Reference:
// https://learn.microsoft.com/en-us/windows/win32/debug/pe-format
// https://en.wikibooks.org/wiki/X86_Disassembly/Windows_Executable_Files

#[derive(Debug, Copy, Clone, Zeroable, Pod)]
#[repr(C)]
struct DOSHeader {
    /// Magic number
    e_magic: [u8; 2],
    /// Bytes on last page of file
    e_cblp: u16,
    /// Pages in file
    e_cp: u16,
    /// Relocations
    e_crlc: u16,
    /// Size of header in paragraphs
    e_cparhdr: u16,
    /// Minimum extra paragraphs needed
    e_minalloc: u16,
    /// Maximum extra paragraphs needed
    e_maxalloc: u16,
    /// Initial (relative) SS value
    e_ss: u16,
    /// Initial SP value
    e_sp: u16,
    /// Checksum
    e_csum: u16,
    /// Initial IP value
    e_ip: u16,
    /// Initial (relative) CS value
    e_cs: u16,
    /// File address of relocation table
    e_lfarlc: u16,
    /// Overlay number
    e_ovno: u16,
    /// Reserved words
    e_res: [u16; 4],
    /// OEM identifier (for e_oeminfo)
    e_oemid: u16,
    /// OEM information; e_oemid specific
    e_oeminfo: u16,
    /// Reserved words
    e_res2: [u16; 10],
    /// File address of new exe header
    e_lfanew: u32,
}

#[derive(Debug, Copy, Clone, Zeroable, Pod)]
#[repr(C)]
struct COFFHeader {
    magic: [u8; 4],
    machine: u16,
    number_of_sections: u16,
    time_date_stamp: u32,
    pointer_to_symbol_table: u32,
    number_of_symbols: u32,
    size_of_optional_header: u16,
    characteristics: u16,
}

#[derive(Debug, Copy, Clone, Zeroable, Pod)]
#[repr(C)]
struct OptionalCOFFHeader {
    magic: u16,
    major_linker_version: u8,
    minor_linker_version: u8,
    size_of_code: u32,
    size_of_initialized_data: u32,
    size_of_uninitialized_data: u32,
    address_of_entry_point: u32,
    base_of_code: u32,
    image_base_or_base_of_data: u64,
    section_alignment: u32,
    file_alignment: u32,
    major_operating_system_version: u16,
    minor_operating_system_version: u16,
    major_image_version: u16,
    minor_image_version: u16,
    major_subsystem_version: u16,
    minor_subsystem_version: u16,
    win32_version_value: u32,
    size_of_image: u32,
    size_of_headers: u32,
    checksum: u32,
    subsystem: u16,
    dll_characteristics: u16,
    // There's more but those vary depending on whether it's PE or PE+.
}

#[derive(Debug, Copy, Clone, Zeroable, Pod, Default)]
#[repr(C)]
struct ExportedSymbolsTableDef {
    _unk: [u8; 0x14],
    number_of_functions: u32,
    number_of_names: u32,
    function_address_array_index: u32,
    function_name_array_index: u32,
    name_ordinals_array_index: u32,
}

/// The machine type (architecture) of a module in a process. An image file can
/// be run only on the specified machine or on a system that emulates the
/// specified machine.
///
/// [Microsoft
/// Documentation](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format#machine-types)
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct MachineType(u16);

impl MachineType {
    /// Reads the machine type of a module (`exe` or `dll`) from the given
    /// process.
    pub fn read(process: &Process, module_address: impl Into<Address>) -> Option<Self> {
        let module_address: Address = module_address.into();

        let (coff_header, _) = read_coff_header(process, module_address)?;

        Some(Self(coff_header.machine))
    }
}

impl fmt::Debug for MachineType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match *self {
            Self::ALPHA => "Alpha AXP, 32-bit address space",
            Self::ALPHA64 => "Alpha 64, 64-bit address space",
            Self::AM33 => "Matsushita AM33",
            Self::AMD64 => "x64",
            Self::ARM => "ARM little endian",
            Self::ARM64 => "ARM64 little endian",
            Self::ARMNT => "ARM Thumb-2 little endian",
            Self::EBC => "EFI byte code",
            Self::I386 => "Intel 386 or later processors and compatible processors",
            Self::IA64 => "Intel Itanium processor family",
            Self::LOONGARCH32 => "LoongArch 32-bit processor family",
            Self::LOONGARCH64 => "LoongArch 64-bit processor family",
            Self::M32R => "Mitsubishi M32R little endian",
            Self::MIPS16 => "MIPS16",
            Self::MIPSFPU => "MIPS with FPU",
            Self::MIPSFPU16 => "MIPS16 with FPU",
            Self::POWERPC => "Power PC little endian",
            Self::POWERPCFP => "Power PC with floating point support",
            Self::R4000 => "MIPS little endian",
            Self::RISCV32 => "RISC-V 32-bit address space",
            Self::RISCV64 => "RISC-V 64-bit address space",
            Self::RISCV128 => "RISC-V 128-bit address space",
            Self::SH3 => "Hitachi SH3",
            Self::SH3DSP => "Hitachi SH3 DSP",
            Self::SH4 => "Hitachi SH4",
            Self::SH5 => "Hitachi SH5",
            Self::THUMB => "Thumb",
            Self::WCEMIPSV2 => "MIPS little-endian WCE v2",
            _ => "Unknown",
        })
    }
}

#[allow(unused)]
impl MachineType {
    /// The content of this field is assumed to be applicable to any machine type
    pub const UNKNOWN: Self = Self(0x0);
    /// Alpha AXP, 32-bit address space
    pub const ALPHA: Self = Self(0x184);
    /// Alpha 64, 64-bit address space
    pub const ALPHA64: Self = Self(0x284);
    /// Matsushita AM33
    pub const AM33: Self = Self(0x1d3);
    /// x64
    pub const AMD64: Self = Self(0x8664);
    /// x64 (Alias for [`AMD64`](Self::AMD64))
    pub const X64: Self = Self::AMD64;
    /// x86-64 (Alias for [`AMD64`](Self::AMD64))
    pub const X86_64: Self = Self::AMD64;
    /// ARM little endian
    pub const ARM: Self = Self(0x1c0);
    /// ARM32 (Alias for [`ARM`](Self::ARM))
    pub const ARM32: Self = Self::ARM;
    /// AArch32 (Alias for [`ARM`](Self::ARM))
    pub const AARCH32: Self = Self::ARM;
    /// ARM64 little endian
    pub const ARM64: Self = Self(0xaa64);
    /// AArch64 (Alias for [`ARM64`](Self::ARM64))
    pub const AARCH64: Self = Self::ARM64;
    /// ARM Thumb-2 little endian
    pub const ARMNT: Self = Self(0x1c4);
    /// AXP 64 (Same as Alpha 64)
    pub const AXP64: Self = Self(0x284);
    /// EFI byte code
    pub const EBC: Self = Self(0xebc);
    /// Intel 386 or later processors and compatible processors
    pub const I386: Self = Self(0x14c);
    /// x86 (Alias for [`I386`](Self::I386))
    pub const X86: Self = Self::I386;
    /// Intel Itanium processor family
    pub const IA64: Self = Self(0x200);
    /// LoongArch 32-bit processor family
    pub const LOONGARCH32: Self = Self(0x6232);
    /// LoongArch 64-bit processor family
    pub const LOONGARCH64: Self = Self(0x6264);
    /// Mitsubishi M32R little endian
    pub const M32R: Self = Self(0x9041);
    /// MIPS16
    pub const MIPS16: Self = Self(0x266);
    /// MIPS with FPU
    pub const MIPSFPU: Self = Self(0x366);
    /// MIPS16 with FPU
    pub const MIPSFPU16: Self = Self(0x466);
    /// Power PC little endian
    pub const POWERPC: Self = Self(0x1f0);
    /// Power PC with floating point support
    pub const POWERPCFP: Self = Self(0x1f1);
    /// MIPS little endian
    pub const R4000: Self = Self(0x166);
    /// RISC-V 32-bit address space
    pub const RISCV32: Self = Self(0x5032);
    /// RISC-V 64-bit address space
    pub const RISCV64: Self = Self(0x5064);
    /// RISC-V 128-bit address space
    pub const RISCV128: Self = Self(0x5128);
    /// Hitachi SH3
    pub const SH3: Self = Self(0x1a2);
    /// Hitachi SH3 DSP
    pub const SH3DSP: Self = Self(0x1a3);
    /// Hitachi SH4
    pub const SH4: Self = Self(0x1a6);
    /// Hitachi SH5
    pub const SH5: Self = Self(0x1a8);
    /// Thumb
    pub const THUMB: Self = Self(0x1c2);
    /// MIPS little-endian WCE v2
    pub const WCEMIPSV2: Self = Self(0x169);

    /// Returns the pointer size for the given machine type. Only the most
    /// common machine types are supported.
    pub const fn pointer_size(self) -> Option<PointerSize> {
        Some(match self {
            Self::AMD64 | Self::ARM64 | Self::IA64 => PointerSize::Bit64,
            Self::I386 | Self::ARM => PointerSize::Bit32,
            _ => return None,
        })
    }
}

/// Reads the size of the image of a module (`exe` or `dll`) from the given
/// process. This may be the more accurate size of the module on Linux, as
/// Proton / Wine don't necessarily report the module size correctly.
pub fn read_size_of_image(process: &Process, module_address: impl Into<Address>) -> Option<u32> {
    let module_address: Address = module_address.into();

    let (coff_header, coff_header_address) = read_coff_header(process, module_address)?;

    if (coff_header.size_of_optional_header as usize) < mem::size_of::<OptionalCOFFHeader>() {
        return None;
    }

    let optional_header = process
        .read::<OptionalCOFFHeader>(coff_header_address + mem::size_of::<COFFHeader>() as u64)
        .ok()?;

    Some(optional_header.size_of_image)
}

fn read_coff_header(process: &Process, module_address: Address) -> Option<(COFFHeader, Address)> {
    let dos_header = process.read::<DOSHeader>(module_address).ok()?;

    if dos_header.e_magic != *b"MZ" {
        return None;
    }

    let coff_header_address = module_address + dos_header.e_lfanew.from_le();

    let coff_header = process.read::<COFFHeader>(coff_header_address).ok()?;

    if coff_header.magic != *b"PE\0\0" {
        return None;
    }

    Some((coff_header, coff_header_address))
}

/// A symbol exported into the current module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol {
    /// The address associated with the current symbol
    pub address: Address,
    /// The address storing the name of the current symbol
    name_addr: Address,
}

impl Symbol {
    /// Tries to retrieve the name of the current symbol
    pub fn get_name<const CAP: usize>(
        &self,
        process: &Process,
    ) -> Result<ArrayCString<CAP>, Error> {
        process.read(self.name_addr)
    }
}

/// Recovers and iterates over the exported symbols for a given module.
/// Returns an empty iterator if no symbols are exported into the current module.
pub fn symbols(
    process: &Process,
    module_address: impl Into<Address>,
) -> impl DoubleEndedIterator<Item = Symbol> + '_ {
    let address: Address = module_address.into();
    let dos_header = process.read::<DOSHeader>(address);

    let is_64_bit = match dos_header {
        Ok(_) => matches!(
            MachineType::read(process, address),
            Some(MachineType::X86_64)
        ),
        _ => false,
    };

    let export_directory = match dos_header {
        Ok(header) => process
            .read::<[u32; 2]>(address + header.e_lfanew + if is_64_bit { 0x88 } else { 0x78 })
            .ok(),
        _ => None,
    };

    let (symbols_def, export_rva, export_dir_size) = match dos_header {
        Ok(_) => match export_directory {
            Some([0, _]) => None,
            Some([export_dir_rva, export_dir_size]) => process
                .read::<ExportedSymbolsTableDef>(address + export_dir_rva)
                .ok()
                .map(|val| (val, export_dir_rva, export_dir_size)),
            _ => None,
        },
        _ => None,
    }
    .unwrap_or_default();

    (0..symbols_def.number_of_names).filter_map(move |i| {
        let ordinal = process
            .read::<u16>(address + symbols_def.name_ordinals_array_index + i.wrapping_mul(2))
            .ok()
            .map(|val| val as u32)
            .filter(|&val| val < symbols_def.number_of_functions)?;

        let func_rva = process
            .read::<u32>(
                address + symbols_def.function_address_array_index + ordinal.wrapping_mul(4),
            )
            .ok()?;

        if func_rva >= export_rva && func_rva < export_rva + export_dir_size {
            return None;
        }

        Some(Symbol {
            address: address + func_rva,
            name_addr: address
                + process
                    .read::<u32>(
                        address + symbols_def.function_name_array_index + i.wrapping_mul(4),
                    )
                    .ok()?,
        })
    })
}

/// A definition of the version number encoded into a PE module.
///
/// This is split into four 16-bit parts:
/// - `major_version`  = HIWORD(dwFileVersionMS)
/// - `minor_version`  = LOWORD(dwFileVersionMS)
/// - `build_part`     = HIWORD(dwFileVersionLS)
/// - `private_part`   = LOWORD(dwFileVersionLS)
// Reference: https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.fileversioninfo.fileversion
#[allow(missing_docs)]
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod, Default)]
pub struct FileVersion {
    pub minor_version: u16,
    pub major_version: u16,
    pub private_part: u16,
    pub build_part: u16,
}

impl FileVersion {
    /// Reads the numeric file version (major.minor.build.private) from the VERSIONINFO
    /// resource of the PE module starting at the specified memory address.
    /// Returns `None` if the module has no version resource or parsing fails.
    pub fn read(process: &Process, module_address: impl Into<Address>) -> Option<Self> {
        #[repr(C)]
        #[derive(Debug, Copy, Clone, Zeroable, Pod, Default)]
        struct VsFixedFileInfo {
            signature: u32, // must be 0xFEEF04BD
            struct_version: u32,
            file_version: FileVersion,
        }

        fn read_dir_header(process: &Process, addr: Address) -> Option<u16> {
            // [6] = NumberOfNamedEntries, [7] = NumberOfIdEntries
            process
                .read::<[u16; 8]>(addr)
                .map(|val| val[6] + val[7])
                .ok()
        }

        #[repr(C)]
        #[derive(Pod, Zeroable, Copy, Clone, Debug, Default)]
        struct DataEntry {
            id: u32,
            offset: u32,
        }

        impl DataEntry {
            fn is_rt_version(&self) -> bool {
                self.id == 0x10
            }

            fn is_directory(&self) -> bool {
                (self.offset & 0x80000000) != 0
            }

            fn get_offset(&self) -> u32 {
                self.offset & 0x7FFFFFFF
            }
        }

        let address: Address = module_address.into();

        let coff_header_address = read_coff_header(process, address)
            .filter(|(coff, _)| {
                (coff.size_of_optional_header as usize) >= mem::size_of::<OptionalCOFFHeader>()
            })
            .map(|(_, address)| address)?;

        let optional_header_address = coff_header_address + mem::size_of::<COFFHeader>() as u64;

        let optional_header_magic = process.read::<u16>(optional_header_address).ok()?;
        let is_64_bit = match optional_header_magic {
            0x10B => false,   // PE32
            0x20B => true,    // PE32+
            _ => return None, // Invalid data
        };

        let res_dd_offset = if is_64_bit { 0x80 } else { 0x70 };
        let res_dd_addr = optional_header_address + res_dd_offset;

        let data_directory = process
            .read::<[u32; 2]>(res_dd_addr)
            .ok()
            .filter(|[a, b]| *a != 0 && *b != 0)
            .map(|[a, _]| a)?;

        let res_base = address + data_directory;

        // Level 1 (resource type = RT_VERSION = 0x10)
        let type_dir = (0..read_dir_header(process, res_base)?)
            .filter_map(|i| process.read::<DataEntry>(res_base + 0x10 + i * 8).ok())
            .find(|entry| entry.is_directory() && entry.is_rt_version())
            .map(|entry| res_base + entry.get_offset())?;

        let lang_dir = (0..read_dir_header(process, type_dir)?)
            .filter_map(|i| process.read::<DataEntry>(type_dir + 0x10 + i * 8).ok())
            .find(|entry| entry.is_directory())
            .map(|entry| res_base + entry.get_offset())?;

        let data_entry = (0..read_dir_header(process, lang_dir)?)
            .filter_map(|i| process.read::<DataEntry>(lang_dir + 0x10 + i * 8).ok())
            .find(|entry| !entry.is_directory())
            .map(|entry| res_base + entry.get_offset())?;

        let vs_version_va = address + process.read::<u32>(data_entry).ok()?;

        process
            .read::<VsFixedFileInfo>(vs_version_va + 0x28)
            .ok()
            .filter(|val| val.signature == 0xFEEF04BD)
            .map(|val| val.file_version)
    }
}
