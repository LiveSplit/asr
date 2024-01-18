//! Support for parsing ELF files (Executable and Linking Format).

use core::{
    fmt,
    iter::{self, FusedIterator},
    mem::{self, size_of},
};

use bytemuck::{Pod, Zeroable};

use crate::{string::ArrayCString, Address, Endian, Error, FromEndian, Process};

// Based on:
// https://refspecs.linuxfoundation.org/elf/elf.pdf

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Header {
    ident: Identification,
    ty: u16,      // 1 = relocatable, 2 = executable, 3 = shared object, 4 = core
    machine: u16, // 0x3e = x86-64
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Identification {
    magic: [u8; 4],  // 0x7f, 'E', 'L', 'F'
    class: u8,       // 32 or 64
    data: u8,        // little or big endian
    version: u8,     // 1
    os_abi: u8,      // 0
    abi_version: u8, // 0
    _padding: [u8; 7],
}

/// Describes information about an ELF file.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Info {
    /// The bitness of the ELF file.
    pub bitness: Bitness,
    /// The endianness of the ELF file.
    pub endian: Endian,
    /// The architecture of the ELF file.
    pub arch: Architecture,
}

impl Info {
    /// Parses the ELF file information from the given data.
    pub fn parse(data: &[u8]) -> Option<Self> {
        let header: &Header = bytemuck::from_bytes(data.get(..mem::size_of::<Header>())?);

        if header.ident.magic != *b"\x7fELF" {
            return None;
        }

        let endian = match header.ident.data {
            1 => Endian::Little,
            2 => Endian::Big,
            _ => return None,
        };

        Some(Self {
            bitness: Bitness(header.ident.class),
            endian,
            arch: Architecture(header.machine.from_endian(endian)),
        })
    }
}

/// The bitness of an ELF file.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Bitness(u8);

impl fmt::Debug for Bitness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match *self {
            Self::BITNESS_32 => "32-bit",
            Self::BITNESS_64 => "64-bit",
            _ => "Unknown",
        })
    }
}

#[allow(unused)]
impl Bitness {
    /// 32-bit
    pub const BITNESS_32: Self = Self(1);
    /// 64-bit
    pub const BITNESS_64: Self = Self(2);

    /// Checks whether the bitness is 32-bit.
    pub fn is_32(self) -> bool {
        self == Self::BITNESS_32
    }

    /// Checks whether the bitness is 64-bit.
    pub fn is_64(self) -> bool {
        self == Self::BITNESS_64
    }
}

/// Segment type identifier for the ELF program header
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct SegmentType(u32);

#[allow(unused)]
impl SegmentType {
    /// Unused header table entry
    pub const PT_NULL: Self = Self(0);
    /// Loadable segment
    pub const PT_LOAD: Self = Self(1);
    /// Dynamic linking information
    pub const PT_DYNAMIC: Self = Self(2);
    /// Interpreter information
    pub const PT_INTERP: Self = Self(3);
    /// Auxiliary information
    pub const PT_NOTE: Self = Self(4);
    /// Reserved
    pub const PT_SHLIB: Self = Self(5);
    /// Segment containing the program header table itself
    pub const PT_PHDR: Self = Self(6);
    /// Thread Local Storage
    pub const PT_TLS: Self = Self(7);
    /// Inclusive range together with PT_HIOS. OS specific
    pub const PT_LOOS: Self = Self(0x60000000);
    /// Inclusive range together with PT_LOOS. OS specific
    pub const PT_HIOS: Self = Self(0x6FFFFFFF);
    /// Inclusive range together with PT_HIPROC. Processor specific
    pub const PT_LOPROC: Self = Self(0x70000000);
    /// Inclusive range together with PT_LOPROC. Processor specific
    pub const PT_HIPROC: Self = Self(0x7FFFFFFF);
}

/// The architecture of an ELF file.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Architecture(u16);

impl fmt::Debug for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Based on:
        // https://sourceware.org/git/?p=binutils-gdb.git;a=blob;f=binutils/readelf.c;h=b872876a8b660be19e1ffc66ee300d0bbfaed345;hb=HEAD#l2746
        f.write_str(match *self {
            Self::EM_NONE => "None",
            Self::EM_M32 => "WE32100",
            Self::EM_SPARC => "Sparc",
            Self::EM_386 => "Intel 80386",
            Self::EM_68K => "MC68000",
            Self::EM_88K => "MC88000",
            Self::EM_IAMCU => "Intel MCU",
            Self::EM_860 => "Intel 80860",
            Self::EM_MIPS => "MIPS R3000",
            Self::EM_S370 => "IBM System/370",
            Self::EM_MIPS_RS3_LE => "MIPS R4000 big-endian",
            Self::EM_OLD_SPARCV9 => "Sparc v9 (old)",
            Self::EM_PARISC => "HPPA",
            Self::EM_VPP550 => "Fujitsu VPP500",
            Self::EM_SPARC32PLUS => "Sparc v8+",
            Self::EM_960 => "Intel 80960",
            Self::EM_PPC => "PowerPC",
            Self::EM_PPC64 => "PowerPC64",
            Self::EM_S390_OLD | Self::EM_S390 => "IBM S/390",
            Self::EM_SPU => "SPU",
            Self::EM_V800 => "Renesas V850 (using RH850 ABI)",
            Self::EM_FR20 => "Fujitsu FR20",
            Self::EM_RH32 => "TRW RH32",
            Self::EM_MCORE => "MCORE",
            Self::EM_ARM => "ARM",
            Self::EM_OLD_ALPHA => "Digital Alpha (old)",
            Self::EM_SH => "Renesas / SuperH SH",
            Self::EM_SPARCV9 => "Sparc v9",
            Self::EM_TRICORE => "Siemens Tricore",
            Self::EM_ARC => "ARC",
            Self::EM_H8_300 => "Renesas H8/300",
            Self::EM_H8_300H => "Renesas H8/300H",
            Self::EM_H8S => "Renesas H8S",
            Self::EM_H8_500 => "Renesas H8/500",
            Self::EM_IA_64 => "Intel IA-64",
            Self::EM_MIPS_X => "Stanford MIPS-X",
            Self::EM_COLDFIRE => "Motorola Coldfire",
            Self::EM_68HC12 => "Motorola MC68HC12 Microcontroller",
            Self::EM_MMA => "Fujitsu Multimedia Accelerator",
            Self::EM_PCP => "Siemens PCP",
            Self::EM_NCPU => "Sony nCPU embedded RISC processor",
            Self::EM_NDR1 => "Denso NDR1 microprocesspr",
            Self::EM_STARCORE => "Motorola Star*Core processor",
            Self::EM_ME16 => "Toyota ME16 processor",
            Self::EM_ST100 => "STMicroelectronics ST100 processor",
            Self::EM_TINYJ => "Advanced Logic Corp. TinyJ embedded processor",
            Self::EM_X86_64 => "Advanced Micro Devices X86-64",
            Self::EM_PDSP => "Sony DSP processor",
            Self::EM_PDP10 => "Digital Equipment Corp. PDP-10",
            Self::EM_PDP11 => "Digital Equipment Corp. PDP-11",
            Self::EM_FX66 => "Siemens FX66 microcontroller",
            Self::EM_ST9PLUS => "STMicroelectronics ST9+ 8/16 bit microcontroller",
            Self::EM_ST7 => "STMicroelectronics ST7 8-bit microcontroller",
            Self::EM_68HC16 => "Motorola MC68HC16 Microcontroller",
            Self::EM_68HC11 => "Motorola MC68HC11 Microcontroller",
            Self::EM_68HC08 => "Motorola MC68HC08 Microcontroller",
            Self::EM_68HC05 => "Motorola MC68HC05 Microcontroller",
            Self::EM_SVX => "Silicon Graphics SVx",
            Self::EM_ST19 => "STMicroelectronics ST19 8-bit microcontroller",
            Self::EM_VAX => "Digital VAX",
            Self::EM_CRIS => "Axis Communications 32-bit embedded processor",
            Self::EM_JAVELIN => "Infineon Technologies 32-bit embedded cpu",
            Self::EM_FIREPATH => "Element 14 64-bit DSP processor",
            Self::EM_ZSP => "LSI Logic's 16-bit DSP processor",
            Self::EM_MMIX => "Donald Knuth's educational 64-bit processor",
            Self::EM_HUANY => "Harvard Universitys's machine-independent object format",
            Self::EM_PRISM => "Vitesse Prism",
            Self::EM_AVR_OLD | Self::EM_AVR => "Atmel AVR 8-bit microcontroller",
            Self::EM_CYGNUS_FR30 | Self::EM_FR30 => "Fujitsu FR30",
            Self::EM_CYGNUS_D10V | Self::EM_D10V => "d10v",
            Self::EM_CYGNUS_D30V | Self::EM_D30V => "d30v",
            Self::EM_CYGNUS_V850 | Self::EM_V850 => "Renesas V850",
            Self::EM_CYGNUS_M32R | Self::EM_M32R => "Renesas M32R (formerly Mitsubishi M32r)",
            Self::EM_CYGNUS_MN10300 | Self::EM_MN10300 => "mn10300",
            Self::EM_CYGNUS_MN10200 | Self::EM_MN10200 => "mn10200",
            Self::EM_PJ => "picoJava",
            Self::EM_OR1K => "OpenRISC 1000",
            Self::EM_ARC_COMPACT => "ARCompact",
            Self::EM_XTENSA_OLD | Self::EM_XTENSA => "Tensilica Xtensa Processor",
            Self::EM_VIDEOCORE => "Alphamosaic VideoCore processor",
            Self::EM_TMM_GPP => "Thompson Multimedia General Purpose Processor",
            Self::EM_NS32K => "National Semiconductor 32000 series",
            Self::EM_TPC => "Tenor Network TPC processor",
            Self::EM_SNP1K => "Trebia SNP 1000 processor",
            Self::EM_ST200 => "STMicroelectronics ST200 microcontroller",
            Self::EM_IP2K_OLD | Self::EM_IP2K => "Ubicom IP2xxx 8-bit microcontrollers",
            Self::EM_MAX => "MAX Processor",
            Self::EM_CR => "National Semiconductor CompactRISC",
            Self::EM_F2MC16 => "Fujitsu F2MC16",
            Self::EM_MSP430 => "Texas Instruments msp430 microcontroller",
            Self::EM_BLACKFIN => "Analog Devices Blackfin",
            Self::EM_SE_C33 => "S1C33 Family of Seiko Epson processors",
            Self::EM_SEP => "Sharp embedded microprocessor",
            Self::EM_ARCA => "Arca RISC microprocessor",
            Self::EM_UNICORE => "Unicore",
            Self::EM_EXCESS => "eXcess 16/32/64-bit configurable embedded CPU",
            Self::EM_DXP => "Icera Semiconductor Inc. Deep Execution Processor",
            Self::EM_ALTERA_NIOS2 => "Altera Nios II",
            Self::EM_CRX => "National Semiconductor CRX microprocessor",
            Self::EM_XGATE => "Motorola XGATE embedded processor",
            Self::EM_C166 | Self::EM_XC16X => "Infineon Technologies xc16x",
            Self::EM_M16C => "Renesas M16C series microprocessors",
            Self::EM_DSPIC30F => "Microchip Technology dsPIC30F Digital Signal Controller",
            Self::EM_CE => "Freescale Communication Engine RISC core",
            Self::EM_M32C => "Renesas M32c",
            Self::EM_TSK3000 => "Altium TSK3000 core",
            Self::EM_RS08 => "Freescale RS08 embedded processor",
            Self::EM_ECOG2 => "Cyan Technology eCOG2 microprocessor",
            Self::EM_SCORE => "SUNPLUS S+Core",
            Self::EM_DSP24 => "New Japan Radio (NJR) 24-bit DSP Processor",
            Self::EM_VIDEOCORE3 => "Broadcom VideoCore III processor",
            Self::EM_LATTICEMICO32 => "Lattice Mico32",
            Self::EM_SE_C17 => "Seiko Epson C17 family",
            Self::EM_TI_C6000 => "Texas Instruments TMS320C6000 DSP family",
            Self::EM_TI_C2000 => "Texas Instruments TMS320C2000 DSP family",
            Self::EM_TI_C5500 => "Texas Instruments TMS320C55x DSP family",
            Self::EM_TI_PRU => "TI PRU I/O processor",
            Self::EM_MMDSP_PLUS => "STMicroelectronics 64bit VLIW Data Signal Processor",
            Self::EM_CYPRESS_M8C => "Cypress M8C microprocessor",
            Self::EM_R32C => "Renesas R32C series microprocessors",
            Self::EM_TRIMEDIA => "NXP Semiconductors TriMedia architecture family",
            Self::EM_QDSP6 => "QUALCOMM DSP6 Processor",
            Self::EM_8051 => "Intel 8051 and variants",
            Self::EM_STXP7X => "STMicroelectronics STxP7x family",
            Self::EM_NDS32 => "Andes Technology compact code size embedded RISC processor family",
            Self::EM_ECOG1X => "Cyan Technology eCOG1X family",
            Self::EM_MAXQ30 => "Dallas Semiconductor MAXQ30 Core microcontrollers",
            Self::EM_XIMO16 => "New Japan Radio (NJR) 16-bit DSP Processor",
            Self::EM_MANIK => "M2000 Reconfigurable RISC Microprocessor",
            Self::EM_CRAYNV2 => "Cray Inc. NV2 vector architecture",
            Self::EM_RX => "Renesas RX",
            Self::EM_METAG => "Imagination Technologies Meta processor architecture",
            Self::EM_MCST_ELBRUS => "MCST Elbrus general purpose hardware architecture",
            Self::EM_ECOG16 => "Cyan Technology eCOG16 family",
            Self::EM_CR16 | Self::EM_MICROBLAZE | Self::EM_MICROBLAZE_OLD => "Xilinx MicroBlaze",
            Self::EM_ETPU => "Freescale Extended Time Processing Unit",
            Self::EM_SLE9X => "Infineon Technologies SLE9X core",
            Self::EM_L1OM => "Intel L1OM",
            Self::EM_K1OM => "Intel K1OM",
            Self::EM_INTEL182 => "Intel (reserved)",
            Self::EM_AARCH64 => "AArch64",
            Self::EM_ARM184 => "ARM (reserved)",
            Self::EM_AVR32 => "Atmel Corporation 32-bit microprocessor",
            Self::EM_STM8 => "STMicroeletronics STM8 8-bit microcontroller",
            Self::EM_TILE64 => "Tilera TILE64 multicore architecture family",
            Self::EM_TILEPRO => "Tilera TILEPro multicore architecture family",
            Self::EM_CUDA => "NVIDIA CUDA architecture",
            Self::EM_TILEGX => "Tilera TILE-Gx multicore architecture family",
            Self::EM_CLOUDSHIELD => "CloudShield architecture family",
            Self::EM_COREA_1ST => "KIPO-KAIST Core-A 1st generation processor family",
            Self::EM_COREA_2ND => "KIPO-KAIST Core-A 2nd generation processor family",
            Self::EM_ARC_COMPACT2 => "ARCv2",
            Self::EM_OPEN8 => "Open8 8-bit RISC soft processor core",
            Self::EM_RL78 => "Renesas RL78",
            Self::EM_VIDEOCORE5 => "Broadcom VideoCore V processor",
            Self::EM_78K0R => "Renesas 78K0R",
            Self::EM_56800EX => "Freescale 56800EX Digital Signal Controller (DSC)",
            Self::EM_BA1 => "Beyond BA1 CPU architecture",
            Self::EM_BA2 => "Beyond BA2 CPU architecture",
            Self::EM_XCORE => "XMOS xCORE processor family",
            Self::EM_MCHP_PIC => "Microchip 8-bit PIC(r) family",
            Self::EM_INTELGT => "Intel Graphics Technology",
            Self::EM_KM32 => "KM211 KM32 32-bit processor",
            Self::EM_KMX32 => "KM211 KMX32 32-bit processor",
            Self::EM_KMX16 => "KM211 KMX16 16-bit processor",
            Self::EM_KMX8 => "KM211 KMX8 8-bit processor",
            Self::EM_KVARC => "KM211 KVARC processor",
            Self::EM_CDP => "Paneve CDP architecture family",
            Self::EM_COGE => "Cognitive Smart Memory Processor",
            Self::EM_COOL => "Bluechip Systems CoolEngine",
            Self::EM_NORC => "Nanoradio Optimized RISC",
            Self::EM_CSR_KALIMBA => "CSR Kalimba architecture family",
            Self::EM_Z80 => "Zilog Z80",
            Self::EM_VISIUM => "CDS VISIUMcore processor",
            Self::EM_FT32 => "FTDI Chip FT32",
            Self::EM_MOXIE => "Moxie",
            Self::EM_AMDGPU => "AMD GPU",
            Self::EM_RISCV => "RISC-V",
            Self::EM_LANAI => "Lanai 32-bit processor",
            Self::EM_CEVA => "CEVA Processor Architecture Family",
            Self::EM_CEVA_X2 => "CEVA X2 Processor Family",
            Self::EM_BPF => "Linux BPF",
            Self::EM_GRAPHCORE_IPU => "Graphcore Intelligent Processing Unit",
            Self::EM_IMG1 => "Imagination Technologies",
            Self::EM_NFP => "Netronome Flow Processor",
            Self::EM_VE => "NEC Vector Engine",
            Self::EM_CSKY => "C-SKY",
            Self::EM_ARC_COMPACT3_64 => "Synopsys ARCv2.3 64-bit",
            Self::EM_MCS6502 => "MOS Technology MCS 6502 processor",
            Self::EM_ARC_COMPACT3 => "Synopsys ARCv2.3 32-bit",
            Self::EM_KVX => "Kalray VLIW core of the MPPA processor family",
            Self::EM_65816 => "WDC 65816/65C816",
            Self::EM_LOONGARCH => "LoongArch",
            Self::EM_KF32 => "ChipON KungFu32",
            Self::EM_MT => "Morpho Techologies MT processor",
            Self::EM_ALPHA => "Alpha",
            Self::EM_WEBASSEMBLY => "Web Assembly",
            Self::EM_DLX => "OpenDLX",
            Self::EM_XSTORMY16 => "Sanyo XStormy16 CPU core",
            Self::EM_IQ2000 => "Vitesse IQ2000",
            Self::EM_M32C_OLD | Self::EM_NIOS32 => "Altera Nios",
            Self::EM_CYGNUS_MEP => "Toshiba MeP Media Engine",
            Self::EM_ADAPTEVA_EPIPHANY => "Adapteva EPIPHANY",
            Self::EM_CYGNUS_FRV => "Fujitsu FR-V",
            Self::EM_S12Z => "Freescale S12Z",
            _ => "Unknown",
        })
    }
}

#[allow(unused)]
impl Architecture {
    // Based on:
    // https://sourceware.org/git/?p=binutils-gdb.git;a=blob;f=include/elf/common.h;h=6f64f05890cc6feba0e9d518abf73e6187d903b0;hb=HEAD#l110

    /// No machine
    pub const EM_NONE: Self = Self(0);
    /// AT&T WE 32100
    pub const EM_M32: Self = Self(1);
    /// SUN SPARC
    pub const EM_SPARC: Self = Self(2);
    /// Intel 80386
    pub const EM_386: Self = Self(3);
    /// Motorola m68k family
    pub const EM_68K: Self = Self(4);
    /// Motorola m88k family
    pub const EM_88K: Self = Self(5);
    /// Intel MCU
    pub const EM_IAMCU: Self = Self(6);
    /// Intel 80860
    pub const EM_860: Self = Self(7);
    /// MIPS R3000 (officially, big-endian only)
    pub const EM_MIPS: Self = Self(8);
    /// IBM System/370
    pub const EM_S370: Self = Self(9);
    /// MIPS R3000 little-endian (Oct 4 1999 Draft).  Deprecated.
    pub const EM_MIPS_RS3_LE: Self = Self(10);
    /// Old version of Sparc v9, from before the ABI.  Deprecated.
    pub const EM_OLD_SPARCV9: Self = Self(11);
    /// HPPA
    pub const EM_PARISC: Self = Self(15);
    /// Old version of PowerPC.  Deprecated.
    pub const EM_PPC_OLD: Self = Self(17);
    /// Fujitsu VPP500
    pub const EM_VPP550: Self = Self(17);
    /// Sun's "v8plus"
    pub const EM_SPARC32PLUS: Self = Self(18);
    /// Intel 80960
    pub const EM_960: Self = Self(19);
    /// PowerPC
    pub const EM_PPC: Self = Self(20);
    /// 64-bit PowerPC
    pub const EM_PPC64: Self = Self(21);
    /// IBM S/390
    pub const EM_S390: Self = Self(22);
    /// Sony/Toshiba/IBM SPU
    pub const EM_SPU: Self = Self(23);
    /// NEC V800 series
    pub const EM_V800: Self = Self(36);
    /// Fujitsu FR20
    pub const EM_FR20: Self = Self(37);
    /// TRW RH32
    pub const EM_RH32: Self = Self(38);
    /// Motorola M*Core
    /// May also be taken by Fujitsu MMA
    pub const EM_MCORE: Self = Self(39);
    /// Old name for MCore
    pub const EM_RCE: Self = Self(39);
    /// ARM
    pub const EM_ARM: Self = Self(40);
    /// Digital Alpha
    pub const EM_OLD_ALPHA: Self = Self(41);
    /// Renesas (formerly Hitachi) / SuperH SH
    pub const EM_SH: Self = Self(42);
    /// SPARC v9 64-bit
    pub const EM_SPARCV9: Self = Self(43);
    /// Siemens Tricore embedded processor
    pub const EM_TRICORE: Self = Self(44);
    /// ARC Cores
    pub const EM_ARC: Self = Self(45);
    /// Renesas (formerly Hitachi) H8/300
    pub const EM_H8_300: Self = Self(46);
    /// Renesas (formerly Hitachi) H8/300H
    pub const EM_H8_300H: Self = Self(47);
    /// Renesas (formerly Hitachi) H8S
    pub const EM_H8S: Self = Self(48);
    /// Renesas (formerly Hitachi) H8/500
    pub const EM_H8_500: Self = Self(49);
    /// Intel IA-64 Processor
    pub const EM_IA_64: Self = Self(50);
    /// Stanford MIPS-X
    pub const EM_MIPS_X: Self = Self(51);
    /// Motorola Coldfire
    pub const EM_COLDFIRE: Self = Self(52);
    /// Motorola M68HC12
    pub const EM_68HC12: Self = Self(53);
    /// Fujitsu Multimedia Accelerator
    pub const EM_MMA: Self = Self(54);
    /// Siemens PCP
    pub const EM_PCP: Self = Self(55);
    /// Sony nCPU embedded RISC processor
    pub const EM_NCPU: Self = Self(56);
    /// Denso NDR1 microprocessor
    pub const EM_NDR1: Self = Self(57);
    /// Motorola Star*Core processor
    pub const EM_STARCORE: Self = Self(58);
    /// Toyota ME16 processor
    pub const EM_ME16: Self = Self(59);
    /// STMicroelectronics ST100 processor
    pub const EM_ST100: Self = Self(60);
    /// Advanced Logic Corp. TinyJ embedded processor
    pub const EM_TINYJ: Self = Self(61);
    /// Advanced Micro Devices X86-64 processor
    pub const EM_X86_64: Self = Self(62);
    /// Sony DSP Processor
    pub const EM_PDSP: Self = Self(63);
    /// Digital Equipment Corp. PDP-10
    pub const EM_PDP10: Self = Self(64);
    /// Digital Equipment Corp. PDP-11
    pub const EM_PDP11: Self = Self(65);
    /// Siemens FX66 microcontroller
    pub const EM_FX66: Self = Self(66);
    /// STMicroelectronics ST9+ 8/16 bit microcontroller
    pub const EM_ST9PLUS: Self = Self(67);
    /// STMicroelectronics ST7 8-bit microcontroller
    pub const EM_ST7: Self = Self(68);
    /// Motorola MC68HC16 Microcontroller
    pub const EM_68HC16: Self = Self(69);
    /// Motorola MC68HC11 Microcontroller
    pub const EM_68HC11: Self = Self(70);
    /// Motorola MC68HC08 Microcontroller
    pub const EM_68HC08: Self = Self(71);
    /// Motorola MC68HC05 Microcontroller
    pub const EM_68HC05: Self = Self(72);
    /// Silicon Graphics SVx
    pub const EM_SVX: Self = Self(73);
    /// STMicroelectronics ST19 8-bit cpu
    pub const EM_ST19: Self = Self(74);
    /// Digital VAX
    pub const EM_VAX: Self = Self(75);
    /// Axis Communications 32-bit embedded processor
    pub const EM_CRIS: Self = Self(76);
    /// Infineon Technologies 32-bit embedded cpu
    pub const EM_JAVELIN: Self = Self(77);
    /// Element 14 64-bit DSP processor
    pub const EM_FIREPATH: Self = Self(78);
    /// LSI Logic's 16-bit DSP processor
    pub const EM_ZSP: Self = Self(79);
    /// Donald Knuth's educational 64-bit processor
    pub const EM_MMIX: Self = Self(80);
    /// Harvard's machine-independent format
    pub const EM_HUANY: Self = Self(81);
    /// SiTera Prism
    pub const EM_PRISM: Self = Self(82);
    /// Atmel AVR 8-bit microcontroller
    pub const EM_AVR: Self = Self(83);
    /// Fujitsu FR30
    pub const EM_FR30: Self = Self(84);
    /// Mitsubishi D10V
    pub const EM_D10V: Self = Self(85);
    /// Mitsubishi D30V
    pub const EM_D30V: Self = Self(86);
    /// Renesas V850 (formerly NEC V850)
    pub const EM_V850: Self = Self(87);
    /// Renesas M32R (formerly Mitsubishi M32R)
    pub const EM_M32R: Self = Self(88);
    /// Matsushita MN10300
    pub const EM_MN10300: Self = Self(89);
    /// Matsushita MN10200
    pub const EM_MN10200: Self = Self(90);
    /// picoJava
    pub const EM_PJ: Self = Self(91);
    /// OpenRISC 1000 32-bit embedded processor
    pub const EM_OR1K: Self = Self(92);
    /// ARC International ARCompact processor
    pub const EM_ARC_COMPACT: Self = Self(93);
    /// Tensilica Xtensa Architecture
    pub const EM_XTENSA: Self = Self(94);
    /// Old Sunplus S+core7 backend magic number. Written in the absence of an ABI.
    pub const EM_SCORE_OLD: Self = Self(95);
    /// Alphamosaic VideoCore processor
    pub const EM_VIDEOCORE: Self = Self(95);
    /// Thompson Multimedia General Purpose Processor
    pub const EM_TMM_GPP: Self = Self(96);
    /// National Semiconductor 32000 series
    pub const EM_NS32K: Self = Self(97);
    /// Tenor Network TPC processor
    pub const EM_TPC: Self = Self(98);
    /// Old value for picoJava.  Deprecated.
    pub const EM_PJ_OLD: Self = Self(99);
    /// Trebia SNP 1000 processor
    pub const EM_SNP1K: Self = Self(99);
    /// STMicroelectronics ST200 microcontroller
    pub const EM_ST200: Self = Self(100);
    /// Ubicom IP2022 micro controller
    pub const EM_IP2K: Self = Self(101);
    /// MAX Processor
    pub const EM_MAX: Self = Self(102);
    /// National Semiconductor CompactRISC
    pub const EM_CR: Self = Self(103);
    /// Fujitsu F2MC16
    pub const EM_F2MC16: Self = Self(104);
    /// TI msp430 micro controller
    pub const EM_MSP430: Self = Self(105);
    /// ADI Blackfin
    pub const EM_BLACKFIN: Self = Self(106);
    /// S1C33 Family of Seiko Epson processors
    pub const EM_SE_C33: Self = Self(107);
    /// Sharp embedded microprocessor
    pub const EM_SEP: Self = Self(108);
    /// Arca RISC Microprocessor
    pub const EM_ARCA: Self = Self(109);
    /// Microprocessor series from PKU-Unity Ltd. and MPRC of Peking University
    pub const EM_UNICORE: Self = Self(110);
    /// eXcess: 16/32/64-bit configurable embedded CPU
    pub const EM_EXCESS: Self = Self(111);
    /// Icera Semiconductor Inc. Deep Execution Processor
    pub const EM_DXP: Self = Self(112);
    /// Altera Nios II soft-core processor
    pub const EM_ALTERA_NIOS2: Self = Self(113);
    /// National Semiconductor CRX
    pub const EM_CRX: Self = Self(114);
    /// Old, value for National Semiconductor CompactRISC.  Deprecated.
    pub const EM_CR16_OLD: Self = Self(115);
    /// Motorola XGATE embedded processor
    pub const EM_XGATE: Self = Self(115);
    /// Infineon C16x/XC16x processor
    pub const EM_C166: Self = Self(116);
    /// Renesas M16C series microprocessors
    pub const EM_M16C: Self = Self(117);
    /// Microchip Technology dsPIC30F Digital Signal Controller
    pub const EM_DSPIC30F: Self = Self(118);
    /// Freescale Communication Engine RISC core
    pub const EM_CE: Self = Self(119);
    /// Renesas M32C series microprocessors
    pub const EM_M32C: Self = Self(120);
    /// Altium TSK3000 core
    pub const EM_TSK3000: Self = Self(131);
    /// Freescale RS08 embedded processor
    pub const EM_RS08: Self = Self(132);
    /// Cyan Technology eCOG2 microprocessor
    pub const EM_ECOG2: Self = Self(134);
    /// Sunplus Score
    pub const EM_SCORE: Self = Self(135);
    /// Sunplus S+core7 RISC processor
    pub const EM_SCORE7: Self = Self(135);
    /// New Japan Radio (NJR) 24-bit DSP Processor
    pub const EM_DSP24: Self = Self(136);
    /// Broadcom VideoCore III processor
    pub const EM_VIDEOCORE3: Self = Self(137);
    /// RISC processor for Lattice FPGA architecture
    pub const EM_LATTICEMICO32: Self = Self(138);
    /// Seiko Epson C17 family
    pub const EM_SE_C17: Self = Self(139);
    /// Texas Instruments TMS320C6000 DSP family
    pub const EM_TI_C6000: Self = Self(140);
    /// Texas Instruments TMS320C2000 DSP family
    pub const EM_TI_C2000: Self = Self(141);
    /// Texas Instruments TMS320C55x DSP family
    pub const EM_TI_C5500: Self = Self(142);
    /// Texas Instruments Programmable Realtime Unit
    pub const EM_TI_PRU: Self = Self(144);
    /// STMicroelectronics 64bit VLIW Data Signal Processor
    pub const EM_MMDSP_PLUS: Self = Self(160);
    /// Cypress M8C microprocessor
    pub const EM_CYPRESS_M8C: Self = Self(161);
    /// Renesas R32C series microprocessors
    pub const EM_R32C: Self = Self(162);
    /// NXP Semiconductors TriMedia architecture family
    pub const EM_TRIMEDIA: Self = Self(163);
    /// QUALCOMM DSP6 Processor
    pub const EM_QDSP6: Self = Self(164);
    /// Intel 8051 and variants
    pub const EM_8051: Self = Self(165);
    /// STMicroelectronics STxP7x family
    pub const EM_STXP7X: Self = Self(166);
    /// Andes Technology compact code size embedded RISC processor family
    pub const EM_NDS32: Self = Self(167);
    /// Cyan Technology eCOG1X family
    pub const EM_ECOG1: Self = Self(168);
    /// Cyan Technology eCOG1X family
    pub const EM_ECOG1X: Self = Self(168);
    /// Dallas Semiconductor MAXQ30 Core Micro-controllers
    pub const EM_MAXQ30: Self = Self(169);
    /// New Japan Radio (NJR) 16-bit DSP Processor
    pub const EM_XIMO16: Self = Self(170);
    /// M2000 Reconfigurable RISC Microprocessor
    pub const EM_MANIK: Self = Self(171);
    /// Cray Inc. NV2 vector architecture
    pub const EM_CRAYNV2: Self = Self(172);
    /// Renesas RX family
    pub const EM_RX: Self = Self(173);
    /// Imagination Technologies Meta processor architecture
    pub const EM_METAG: Self = Self(174);
    /// MCST Elbrus general purpose hardware architecture
    pub const EM_MCST_ELBRUS: Self = Self(175);
    /// Cyan Technology eCOG16 family
    pub const EM_ECOG16: Self = Self(176);
    /// National Semiconductor CompactRISC 16-bit processor
    pub const EM_CR16: Self = Self(177);
    /// Freescale Extended Time Processing Unit
    pub const EM_ETPU: Self = Self(178);
    /// Infineon Technologies SLE9X core
    pub const EM_SLE9X: Self = Self(179);
    /// Intel L1OM
    pub const EM_L1OM: Self = Self(180);
    /// Intel K1OM
    pub const EM_K1OM: Self = Self(181);
    /// Reserved by Intel
    pub const EM_INTEL182: Self = Self(182);
    /// ARM 64-bit architecture
    pub const EM_AARCH64: Self = Self(183);
    /// Reserved by ARM
    pub const EM_ARM184: Self = Self(184);
    /// Atmel Corporation 32-bit microprocessor family
    pub const EM_AVR32: Self = Self(185);
    /// STMicroeletronics STM8 8-bit microcontroller
    pub const EM_STM8: Self = Self(186);
    /// Tilera TILE64 multicore architecture family
    pub const EM_TILE64: Self = Self(187);
    /// Tilera TILEPro multicore architecture family
    pub const EM_TILEPRO: Self = Self(188);
    /// Xilinx MicroBlaze 32-bit RISC soft processor core
    pub const EM_MICROBLAZE: Self = Self(189);
    /// NVIDIA CUDA architecture
    pub const EM_CUDA: Self = Self(190);
    /// Tilera TILE-Gx multicore architecture family
    pub const EM_TILEGX: Self = Self(191);
    /// CloudShield architecture family
    pub const EM_CLOUDSHIELD: Self = Self(192);
    /// KIPO-KAIST Core-A 1st generation processor family
    pub const EM_COREA_1ST: Self = Self(193);
    /// KIPO-KAIST Core-A 2nd generation processor family
    pub const EM_COREA_2ND: Self = Self(194);
    /// Synopsys ARCompact V2
    pub const EM_ARC_COMPACT2: Self = Self(195);
    /// Open8 8-bit RISC soft processor core
    pub const EM_OPEN8: Self = Self(196);
    /// Renesas RL78 family.
    pub const EM_RL78: Self = Self(197);
    /// Broadcom VideoCore V processor
    pub const EM_VIDEOCORE5: Self = Self(198);
    /// Renesas 78K0R.
    pub const EM_78K0R: Self = Self(199);
    /// Freescale 56800EX Digital Signal Controller (DSC)
    pub const EM_56800EX: Self = Self(200);
    /// Beyond BA1 CPU architecture
    pub const EM_BA1: Self = Self(201);
    /// Beyond BA2 CPU architecture
    pub const EM_BA2: Self = Self(202);
    /// XMOS xCORE processor family
    pub const EM_XCORE: Self = Self(203);
    /// Microchip 8-bit PIC(r) family
    pub const EM_MCHP_PIC: Self = Self(204);
    /// Intel Graphics Technology
    pub const EM_INTELGT: Self = Self(205);
    /// Reserved by Intel
    pub const EM_INTEL206: Self = Self(206);
    /// Reserved by Intel
    pub const EM_INTEL207: Self = Self(207);
    /// Reserved by Intel
    pub const EM_INTEL208: Self = Self(208);
    /// Reserved by Intel
    pub const EM_INTEL209: Self = Self(209);
    /// KM211 KM32 32-bit processor
    pub const EM_KM32: Self = Self(210);
    /// KM211 KMX32 32-bit processor
    pub const EM_KMX32: Self = Self(211);
    /// KM211 KMX16 16-bit processor
    pub const EM_KMX16: Self = Self(212);
    /// KM211 KMX8 8-bit processor
    pub const EM_KMX8: Self = Self(213);
    /// KM211 KVARC processor
    pub const EM_KVARC: Self = Self(214);
    /// Paneve CDP architecture family
    pub const EM_CDP: Self = Self(215);
    /// Cognitive Smart Memory Processor
    pub const EM_COGE: Self = Self(216);
    /// Bluechip Systems CoolEngine
    pub const EM_COOL: Self = Self(217);
    /// Nanoradio Optimized RISC
    pub const EM_NORC: Self = Self(218);
    /// CSR Kalimba architecture family
    pub const EM_CSR_KALIMBA: Self = Self(219);
    /// Zilog Z80
    pub const EM_Z80: Self = Self(220);
    /// Controls and Data Services VISIUMcore processor
    pub const EM_VISIUM: Self = Self(221);
    /// FTDI Chip FT32 high performance 32-bit RISC architecture
    pub const EM_FT32: Self = Self(222);
    /// Moxie processor family
    pub const EM_MOXIE: Self = Self(223);
    /// AMD GPU architecture
    pub const EM_AMDGPU: Self = Self(224);
    /// RISC-V
    pub const EM_RISCV: Self = Self(243);
    /// Lanai 32-bit processor.
    pub const EM_LANAI: Self = Self(244);
    /// CEVA Processor Architecture Family
    pub const EM_CEVA: Self = Self(245);
    /// CEVA X2 Processor Family
    pub const EM_CEVA_X2: Self = Self(246);
    /// Linux BPF â€“ in-kernel virtual machine.
    pub const EM_BPF: Self = Self(247);
    /// Graphcore Intelligent Processing Unit
    pub const EM_GRAPHCORE_IPU: Self = Self(248);
    /// Imagination Technologies
    pub const EM_IMG1: Self = Self(249);
    /// Netronome Flow Processor.
    pub const EM_NFP: Self = Self(250);
    /// NEC Vector Engine
    pub const EM_VE: Self = Self(251);
    /// C-SKY processor family.
    pub const EM_CSKY: Self = Self(252);
    /// Synopsys ARCv2.3 64-bit
    pub const EM_ARC_COMPACT3_64: Self = Self(253);
    /// MOS Technology MCS 6502 processor
    pub const EM_MCS6502: Self = Self(254);
    /// Synopsys ARCv2.3 32-bit
    pub const EM_ARC_COMPACT3: Self = Self(255);
    /// Kalray VLIW core of the MPPA processor family
    pub const EM_KVX: Self = Self(256);
    /// WDC 65816/65C816
    pub const EM_65816: Self = Self(257);
    /// LoongArch
    pub const EM_LOONGARCH: Self = Self(258);
    /// ChipON KungFu32
    pub const EM_KF32: Self = Self(259);
    /// LAPIS nX-U16/U8
    pub const EM_U16_U8CORE: Self = Self(260);
    /// Tachyum
    pub const EM_TACHYUM: Self = Self(261);
    /// NXP 56800EF Digital Signal Controller (DSC)
    pub const EM_56800EF: Self = Self(262);

    /// AVR magic number.  Written in the absense of an ABI.
    pub const EM_AVR_OLD: Self = Self(0x1057);

    /// MSP430 magic number.  Written in the absense of everything.
    pub const EM_MSP430_OLD: Self = Self(0x1059);

    /// Morpho MT.   Written in the absense of an ABI.
    pub const EM_MT: Self = Self(0x2530);

    /// FR30 magic number - no EABI available.
    pub const EM_CYGNUS_FR30: Self = Self(0x3330);

    /// Unofficial value for Web Assembly binaries, as used by LLVM.
    pub const EM_WEBASSEMBLY: Self = Self(0x4157);

    /// Freescale S12Z.   The Freescale toolchain generates elf files with this value.
    pub const EM_S12Z: Self = Self(0x4DEF);

    /// DLX magic number.  Written in the absense of an ABI.
    pub const EM_DLX: Self = Self(0x5aa5);

    /// FRV magic number - no EABI available??.
    pub const EM_CYGNUS_FRV: Self = Self(0x5441);

    /// Infineon Technologies 16-bit microcontroller with C166-V2 core.
    pub const EM_XC16X: Self = Self(0x4688);

    /// D10V backend magic number.  Written in the absence of an ABI.
    pub const EM_CYGNUS_D10V: Self = Self(0x7650);

    /// D30V backend magic number.  Written in the absence of an ABI.
    pub const EM_CYGNUS_D30V: Self = Self(0x7676);

    /// Ubicom IP2xxx;   Written in the absense of an ABI.
    pub const EM_IP2K_OLD: Self = Self(0x8217);

    /// Cygnus PowerPC ELF backend.  Written in the absence of an ABI.
    pub const EM_CYGNUS_POWERPC: Self = Self(0x9025);

    /// Alpha backend magic number.  Written in the absence of an ABI.
    pub const EM_ALPHA: Self = Self(0x9026);

    /// Cygnus M32R ELF backend.  Written in the absence of an ABI.
    pub const EM_CYGNUS_M32R: Self = Self(0x9041);

    /// V850 backend magic number.  Written in the absense of an ABI.
    pub const EM_CYGNUS_V850: Self = Self(0x9080);

    /// old S/390 backend magic number. Written in the absence of an ABI.
    pub const EM_S390_OLD: Self = Self(0xa390);

    /// Old, unofficial value for Xtensa.
    pub const EM_XTENSA_OLD: Self = Self(0xabc7);

    /// Sanyo XStormy16 CPU core
    pub const EM_XSTORMY16: Self = Self(0xad45);

    /// mn10300 backend magic numbers.
    /// Written in the absense of an ABI.
    pub const EM_CYGNUS_MN10300: Self = Self(0xbeef);
    /// mn10200 backend magic numbers.
    /// Written in the absense of an ABI.
    pub const EM_CYGNUS_MN10200: Self = Self(0xdead);

    /// Renesas M32C and M16C.
    pub const EM_M32C_OLD: Self = Self(0xFEB0);

    /// Vitesse IQ2000.
    pub const EM_IQ2000: Self = Self(0xFEBA);

    /// NIOS magic number - no EABI available.
    pub const EM_NIOS32: Self = Self(0xFEBB);

    /// Toshiba MeP
    pub const EM_CYGNUS_MEP: Self = Self(0xF00D);

    /// Old, unofficial value for Moxie.
    pub const EM_MOXIE_OLD: Self = Self(0xFEED);

    /// Old MicroBlaze
    pub const EM_MICROBLAZE_OLD: Self = Self(0xbaab);

    /// Adapteva's Epiphany architecture.
    pub const EM_ADAPTEVA_EPIPHANY: Self = Self(0x1223);

    /// Old constant that might be in use by some software.
    pub const EM_OPENRISC: Self = Self::EM_OR1K;

    /// C-SKY historically used 39, the same value as MCORE, from which the
    /// architecture was derived.
    pub const EM_CSKY_OLD: Self = Self::EM_MCORE;
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Elf32 {
    e_ident: Identification,
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u32,
    e_phoff: u32,
    e_shoff: u32,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Elf64 {
    e_ident: Identification,
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

/// Checks if a given ELF module is 64-bit
pub fn is_64_bit(process: &Process, module_address: Address) -> Option<bool> {
    let header = process.read::<Header>(module_address).ok()?;
    let info = Info::parse(bytemuck::bytes_of(&header))?;
    match info.bitness {
        Bitness::BITNESS_64 => Some(true),
        _ => Some(false),
    }
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct ProgramHeader32 {
    p_type: u32,
    p_offset: u32,
    p_vaddr: u32,
    p_paddr: u32,
    p_filesz: u32,
    p_memsz: u32,
    p_flags: u32,
    p_align: u32,
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct ProgramHeader64 {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

/// A symbol exported into the current module.
pub struct Symbol {
    /// The address associated with the current function
    pub address: Address,
    /// The size occupied in memory by the current function
    pub size: u64,
    /// The address storing the name of the current function
    name_addr: Address,
}

impl Symbol {
    /// Tries to retrieve the name of the current function
    pub fn get_name<const CAP: usize>(
        &self,
        process: &Process,
    ) -> Result<ArrayCString<CAP>, Error> {
        process.read(self.name_addr)
    }
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct SymTab64 {
    st_name: u32,
    st_info: u8,
    st_other: u8,
    st_shndx: u16,
    st_value: u64,
    st_size: u64,
}

/// Recovers and iterates over the exported symbols for a given module.
/// Returns an empty iterator if no symbols are exported into the current module.
///
/// By using this function, the user must be aware of the following limitations:
/// - Only allocatable symbols and symbols used by the dynamic linker are exported
/// (.symtab is not loaded in memory at runtime)
/// - Only 64-bit ELFs are supported (an empty iterator will be returned for 32-bit ELFs)
pub fn symbols(
    process: &Process,
    module_address: Address,
) -> impl FusedIterator<Item = Symbol> + '_ {
    let header = process.read::<Elf64>(module_address);

    // Only 64 bit ELF is supported
    let is_64_bit = match header {
        Ok(x) => Info::parse(bytemuck::bytes_of(&x)).is_some_and(|info| info.bitness.is_64()),
        _ => false,
    };

    let e_phnum = match (is_64_bit, &header) {
        (true, Ok(x)) => x.e_phnum,
        _ => 0,
    };

    let e_phoff = match (is_64_bit, &header) {
        (true, Ok(x)) => Some(x.e_phoff),
        _ => None,
    };

    let e_phentsize = match (is_64_bit, &header) {
        (true, Ok(x)) => Some(x.e_phentsize),
        _ => None,
    };

    let mut program_headers = (0..e_phnum).filter_map(|index| {
        process
            .read::<ProgramHeader64>(module_address + e_phoff? + e_phentsize?.wrapping_mul(index))
            .ok()
    });

    let (segment_address, segment_size) = match program_headers
        .find(|p_header| SegmentType(p_header.p_type) == SegmentType::PT_DYNAMIC)
    {
        Some(x) => (Some(x.p_vaddr), x.p_memsz),
        _ => (None, 0),
    };

    let entries = || {
        (0..segment_size)
            .step_by(size_of::<[u64; 2]>())
            .filter_map(|entry| {
                process
                    .read::<[u64; 2]>(module_address + segment_address? + entry)
                    .ok()
            })
    };

    let symtab = entries()
        .find(|val| val[0] == 6)
        .map(|[_, b]| Address::new(b));
    let strtab = entries()
        .find(|val| val[0] == 5)
        .map(|[_, b]| Address::new(b));
    let strsz = entries().find(|val| val[0] == 0xA).map(|[_, b]| b);

    let mut offset = 0;
    iter::from_fn(move || {
        let table = process.read::<SymTab64>(symtab? + offset).ok()?;
        if table.st_name as u64 >= strsz? {
            None
        } else {
            let f_address = module_address + table.st_value;
            let f_size = table.st_size;
            let f_name = strtab? + table.st_name;

            offset += size_of::<SymTab64>() as u64;

            Some(Symbol {
                address: f_address,
                size: f_size,
                name_addr: f_name,
            })
        }
    })
    .fuse()
}
