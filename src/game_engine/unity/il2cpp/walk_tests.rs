//! Tests over a hand-laid image of IL2CPP's structures.

use super::{IL2CPPOffsets, Module, Version};
use crate::runtime::mock::with_process;
use crate::{Address, PointerSize};

use std::vec;

const BASE: u64 = 0x20_0000;

fn put(image: &mut [u8], at: u64, bytes: &[u8]) {
    let at = at as usize;
    image[at..at + bytes.len()].copy_from_slice(bytes);
}

// A player as `attach_auto_detect` sees it: `GameAssembly.dll` with a PE
// header and the x64 code that points at both globals, `UnityPlayer.dll` with a
// PE header and a version resource, and the mapped metadata file when it is
// there yet.
struct Player {
    unity: (u16, u16, u16, u16),
    metadata: Option<u32>,
}

const GAME_ASSEMBLY: u64 = 0x1_8000_0000;
const UNITY_PLAYER: u64 = 0x1900_0000;
const METADATA: u64 = 0x2000_0000;

impl Player {
    fn attach(&self) -> Option<Module> {
        let game_assembly = Self::game_assembly();
        let unity_player = Self::unity_player(self.unity);
        let metadata = self.metadata.map(|version| {
            let mut i = vec![0; 0x100];
            put(&mut i, 0x0, &0xFAB1_1BAF_u32.to_le_bytes());
            put(&mut i, 0x4, &version.to_le_bytes());
            i
        });
        let mut regions = vec![
            (GAME_ASSEMBLY, &game_assembly[..]),
            (UNITY_PLAYER, &unity_player[..]),
        ];
        if let Some(metadata) = &metadata {
            regions.push((METADATA, &metadata[..]));
        }
        crate::runtime::mock::with_modules(
            &regions,
            &[
                ("GameAssembly.dll", GAME_ASSEMBLY, 0x1000),
                ("UnityPlayer.dll", UNITY_PLAYER, 0x1000),
            ],
            Module::attach_auto_detect,
        )
    }

    // A PE32+ header for x64, with the image size and, when asked, the
    // resource directory pointing at `at`.
    fn pe_header(i: &mut [u8], resources_at: Option<u32>) {
        put(i, 0x00, b"MZ");
        put(i, 0x3C, &0x80_u32.to_le_bytes());
        put(i, 0x80, b"PE\0\0");
        put(i, 0x84, &0x8664_u16.to_le_bytes()); // machine: x64
        put(i, 0x94, &0xF0_u16.to_le_bytes()); // size of optional header
        put(i, 0x98, &0x20B_u16.to_le_bytes()); // PE32+
        put(i, 0x98 + 0x38, &0x1000_u32.to_le_bytes()); // size of image
        if let Some(at) = resources_at {
            put(i, 0x98 + 0x80, &at.to_le_bytes());
            put(i, 0x98 + 0x84, &0x100_u32.to_le_bytes());
        }
    }

    fn game_assembly() -> vec::Vec<u8> {
        let mut i = vec![0; 0x1000];
        Self::pe_header(&mut i, None);
        let rel = |from: u64, to: u64| ((to as i64 - (from + 4) as i64) as i32).to_le_bytes();
        let base = GAME_ASSEMBLY;
        // jne; mov rbx, [begin]; cmp rbx, [end]
        put(&mut i, 0x300, &[0x75, 0xF9, 0x48, 0x8B, 0x1D]);
        put(&mut i, 0x305, &rel(base + 0x305, base + 0x900));
        put(&mut i, 0x309, &[0x48, 0x3B, 0x1D]);
        put(&mut i, 0x30C, &rel(base + 0x30C, base + 0x908));
        put(&mut i, 0x400, b"global-metadata.dat\0");
        // lea rcx, [name]; shr rcx, 6; mov [table], rax
        put(&mut i, 0x500, &[0x48, 0x8D, 0x0D]);
        put(&mut i, 0x503, &rel(base + 0x503, base + 0x400));
        put(&mut i, 0x580, &[0x48, 0xC1, 0xE9, 0x06]);
        put(&mut i, 0x5A0, &[0x48, 0x89, 0x05]);
        put(&mut i, 0x5A3, &rel(base + 0x5A3, base + 0x910));
        i
    }

    // The version resource, the way the resource directory lays it: the
    // type directory holds RT_VERSION, which holds one language, which holds
    // the data entry pointing at VS_VERSIONINFO.
    fn unity_player(unity: (u16, u16, u16, u16)) -> vec::Vec<u8> {
        let mut i = vec![0; 0x1000];
        Self::pe_header(&mut i, Some(0x400));
        let entry = |i: &mut [u8], at: u64, id: u32, offset: u32| {
            put(i, at, &id.to_le_bytes());
            put(i, at + 4, &offset.to_le_bytes());
        };
        // root: one id entry, RT_VERSION (0x10), a directory at +0x20
        put(&mut i, 0x400 + 0xE, &1_u16.to_le_bytes());
        entry(&mut i, 0x410, 0x10, 0x8000_0020);
        // type directory: one directory entry at +0x40
        put(&mut i, 0x420 + 0xE, &1_u16.to_le_bytes());
        entry(&mut i, 0x430, 1, 0x8000_0040);
        // language directory: one data entry at +0x60
        put(&mut i, 0x440 + 0xE, &1_u16.to_le_bytes());
        entry(&mut i, 0x450, 0x409, 0x60);
        // the data entry names VS_VERSIONINFO at 0x600
        put(&mut i, 0x460, &0x600_u32.to_le_bytes());
        // VS_FIXEDFILEINFO sits 0x28 in
        put(&mut i, 0x628, &0xFEEF_04BD_u32.to_le_bytes());
        put(&mut i, 0x630, &unity.1.to_le_bytes());
        put(&mut i, 0x632, &unity.0.to_le_bytes());
        put(&mut i, 0x634, &unity.3.to_le_bytes());
        put(&mut i, 0x636, &unity.2.to_le_bytes());
        i
    }
}

const MEASURED_6000_5: (u16, u16, u16, u16) = (6000, 5, 10, 54518);

// A game on a measured player, with its metadata mapped, attaches with the
// offsets measured on that player.
#[test]
fn attach_auto_detect_uses_a_measured_build() {
    let module = Player {
        unity: MEASURED_6000_5,
        metadata: Some(107),
    }
    .attach()
    .unwrap();
    assert_eq!(module.assemblies, Address::new(GAME_ASSEMBLY + 0x900));
    assert_eq!(
        module.type_info_definition_table,
        Address::new(GAME_ASSEMBLY + 0x910)
    );
    assert_eq!(module.offsets.class.static_fields, 0xA0);
}

// The metadata file is mapped after `GameAssembly.dll`. Until it is, a game
// on a measured player has to wait rather than attach with the version
// table's offsets and keep them.
#[test]
fn attach_auto_detect_waits_for_the_metadata_of_a_measured_player() {
    let module = Player {
        unity: MEASURED_6000_5,
        metadata: None,
    }
    .attach();
    assert!(module.is_none());
}

// A player nobody measured takes the version table, whether or not its
// metadata is mapped yet.
#[test]
fn attach_auto_detect_falls_back_for_an_unmeasured_player() {
    for metadata in [None, Some(107)] {
        let module = Player {
            unity: (6000, 5, 11, 1),
            metadata,
        }
        .attach()
        .unwrap();
        assert_eq!(module.offsets.class.static_fields, 0xB8);
    }
}

// A measured player whose metadata says another version is not that build.
#[test]
fn attach_auto_detect_falls_back_when_the_metadata_disagrees() {
    let module = Player {
        unity: MEASURED_6000_5,
        metadata: Some(110),
    }
    .attach()
    .unwrap();
    assert_eq!(module.offsets.class.static_fields, 0xB8);
}

// The x86 code that points at both globals. The assemblies loop reads the vector's
// begin and end by absolute address. The table store follows the string that
// names the metadata file, and has three shapes across Unity versions, so the
// image takes the stores to lay.
struct X86Image {
    stores: vec::Vec<(u64, &'static [u8], u64, u64)>,
    end_operand: u64,
}

const DIVIDED: &[u8] = &[0xC1, 0xEA, 0x06, 0x52, 0xE8, 0, 0, 0, 0, 0xA3];
const DIVIDED_RELOAD: &[u8] = &[
    0xC1, 0xEA, 0x05, 0x52, 0xE8, 0, 0, 0, 0, 0x8B, 0x0D, 0, 0, 0, 0, 0xA3,
];
const PUSHED: &[u8] = &[0xFF, 0xB0, 0xF4, 0, 0, 0, 0xE8, 0, 0, 0, 0, 0xA3];

impl X86Image {
    fn with_store(store: &'static [u8], operand_at: u64) -> Self {
        Self {
            stores: vec![(0x480, store, operand_at, BASE + 0x910)],
            end_operand: BASE + 0x904,
        }
    }

    fn lay(&self) -> vec::Vec<u8> {
        let mut i = vec![0; 0x2000];
        let abs = |target: u64| (target as u32).to_le_bytes();

        // jne; mov esi, [begin]; sub edi, ecx; cmp esi, [end]
        put(&mut i, 0x100, &[0x75, 0xF9, 0x8B, 0x35]);
        put(&mut i, 0x104, &abs(BASE + 0x900));
        put(&mut i, 0x108, &[0x2B, 0xF9, 0x3B, 0x35]);
        put(&mut i, 0x10C, &abs(self.end_operand));

        // push offset "global-metadata.dat"; call
        put(&mut i, 0x300, b"global-metadata.dat\0");
        put(&mut i, 0x400, &[0x68]);
        put(&mut i, 0x401, &abs(BASE + 0x300));
        put(&mut i, 0x405, &[0xE8, 0, 0, 0, 0]);

        for &(at, store, operand_at, target) in &self.stores {
            put(&mut i, at, store);
            put(&mut i, at + operand_at, &abs(target));
        }
        i
    }
}

fn x86_image(store: &'static [u8], operand_at: u64) -> vec::Vec<u8> {
    X86Image::with_store(store, operand_at).lay()
}

// The scan reads no offsets, and the version tables carry no 32 bit arm, so
// the x64 offsets stand in. The module is the first 0x1000 bytes of the
// image, so the image can hold bytes past the module's end.
fn attach_x86(process: &crate::Process) -> Option<Module> {
    Module::attach_with(
        process,
        (Address::new(BASE), 0x1000),
        PointerSize::Bit32,
        Version::V2022,
        IL2CPPOffsets::new(Version::V2022, PointerSize::Bit64).unwrap(),
    )
}

// shr edx, 6; push edx; call; mov [table], eax
#[test]
fn x86_globals_resolve_through_a_divided_count() {
    let i = x86_image(DIVIDED, 10);
    with_process(&[(BASE, &i)], |process| {
        let module = attach_x86(process).unwrap();
        assert_eq!(module.assemblies, Address::new(BASE + 0x900));
        assert_eq!(
            module.type_info_definition_table,
            Address::new(BASE + 0x910)
        );
    });
}

// shr edx, 5; push edx; call; mov ecx, [x]; mov [table], eax
#[test]
fn x86_globals_resolve_through_a_divided_count_and_a_reload() {
    let i = x86_image(DIVIDED_RELOAD, 16);
    with_process(&[(BASE, &i)], |process| {
        let module = attach_x86(process).unwrap();
        assert_eq!(
            module.type_info_definition_table,
            Address::new(BASE + 0x910)
        );
    });
}

// push dword [eax + 0xF4]; call; mov [table], eax
#[test]
fn x86_globals_resolve_through_a_pushed_count() {
    let i = x86_image(PUSHED, 12);
    with_process(&[(BASE, &i)], |process| {
        let module = attach_x86(process).unwrap();
        assert_eq!(
            module.type_info_definition_table,
            Address::new(BASE + 0x910)
        );
    });
}

// The table is the first store after the name, whatever its shape. A later
// store of another shape, like the next table's, must not win.
#[test]
fn x86_table_store_is_the_first_of_any_shape_after_the_name() {
    let image = X86Image {
        stores: vec![
            (0x480, DIVIDED, 10, BASE + 0x910),
            (0x4A0, DIVIDED_RELOAD, 16, BASE + 0x920),
        ],
        end_operand: BASE + 0x904,
    };
    let i = image.lay();
    with_process(&[(BASE, &i)], |process| {
        let module = attach_x86(process).unwrap();
        assert_eq!(
            module.type_info_definition_table,
            Address::new(BASE + 0x910)
        );
    });
}

// A vector's end sits one pointer past its begin. A loop over some other
// pair is not the assemblies.
#[test]
fn x86_assemblies_scan_wants_the_end_beside_the_begin() {
    let image = X86Image {
        stores: vec![(0x480, DIVIDED, 10, BASE + 0x910)],
        end_operand: BASE + 0x930,
    };
    let i = image.lay();
    with_process(&[(BASE, &i)], |process| {
        assert!(attach_x86(process).is_none());
    });
}

// A global the code points at outside the module is not this module's.
#[test]
fn x86_globals_outside_the_module_are_refused() {
    let image = X86Image {
        stores: vec![(0x480, DIVIDED, 10, BASE + 0x1910)],
        end_operand: BASE + 0x904,
    };
    let i = image.lay();
    with_process(&[(BASE, &i)], |process| {
        assert!(attach_x86(process).is_none());
    });
}

// The window after the name stops at the module's end, even when memory
// goes on past it.
#[test]
fn x86_store_scan_stops_at_the_module_end() {
    let image = X86Image {
        stores: vec![(0x1080, DIVIDED, 10, BASE + 0x910)],
        end_operand: BASE + 0x904,
    };
    let mut i = image.lay();
    // Move the name push to the module's last page, so the window reaches
    // past the end.
    put(&mut i, 0x400, &[0; 10]);
    put(&mut i, 0xF80, &[0x68]);
    put(&mut i, 0xF81, &(BASE as u32 + 0x300).to_le_bytes());
    put(&mut i, 0xF85, &[0xE8, 0, 0, 0, 0]);
    with_process(&[(BASE, &i)], |process| {
        assert!(attach_x86(process).is_none());
    });
}

// The x64 code that points at both globals, with displacements from the next
// instruction.
#[test]
fn x64_globals_resolve_from_a_mapped_image() {
    let mut i = vec![0; 0x1000];
    let rel = |from: u64, to: u64| ((to as i64 - (from + 4) as i64) as i32).to_le_bytes();

    // jne; mov rbx, [begin]; cmp rbx, [end]
    put(&mut i, 0x100, &[0x75, 0xF9, 0x48, 0x8B, 0x1D]);
    put(&mut i, 0x105, &rel(BASE + 0x105, BASE + 0x900));
    put(&mut i, 0x109, &[0x48, 0x3B, 0x1D]);
    put(&mut i, 0x10C, &rel(BASE + 0x10C, BASE + 0x908));

    put(&mut i, 0x300, b"global-metadata.dat\0");
    // lea rcx, [name]; ... shr rcx, 6; ... mov [table], rax
    put(&mut i, 0x400, &[0x48, 0x8D, 0x0D]);
    put(&mut i, 0x403, &rel(BASE + 0x403, BASE + 0x300));
    put(&mut i, 0x480, &[0x48, 0xC1, 0xE9, 0x06]);
    put(&mut i, 0x4A0, &[0x48, 0x89, 0x05]);
    put(&mut i, 0x4A3, &rel(BASE + 0x4A3, BASE + 0x910));

    with_process(&[(BASE, &i)], |process| {
        let module = Module::attach_with(
            process,
            (Address::new(BASE), 0x1000),
            PointerSize::Bit64,
            Version::V2022,
            IL2CPPOffsets::new(Version::V2022, PointerSize::Bit64).unwrap(),
        )
        .unwrap();
        assert_eq!(module.assemblies, Address::new(BASE + 0x900));
        assert_eq!(
            module.type_info_definition_table,
            Address::new(BASE + 0x910)
        );
    });
}

// The 64 bit scanner reads displacements, so it must not accept x86 code.
#[test]
fn x64_scanner_refuses_an_x86_image() {
    let i = x86_image(DIVIDED, 10);
    with_process(&[(BASE, &i)], |process| {
        assert!(Module::attach_with(
            process,
            (Address::new(BASE), 0x1000),
            PointerSize::Bit64,
            Version::V2022,
            IL2CPPOffsets::new(Version::V2022, PointerSize::Bit64).unwrap(),
        )
        .is_none());
    });
}

// A 32 bit target lays the assemblies vector and its pointers at four bytes.
#[test]
fn images_resolve_on_32_bit_targets() {
    let mut i = vec![0; 0x1000];
    let ptr = |i: &mut [u8], at: u64, target: u64| {
        put(i, at, &(target as u32).to_le_bytes());
    };

    put(&mut i, 0x800, b"Assembly-CSharp");
    ptr(&mut i, 0x0, BASE + 0x40); // the vector's begin
    ptr(&mut i, 0x4, BASE + 0x44); // and end, one assembly along
    ptr(&mut i, 0x40, BASE + 0x80);
    ptr(&mut i, 0x80, BASE + 0x100); // Il2CppAssembly.image
    ptr(&mut i, 0x80 + 0x18, BASE + 0x800); // Il2CppAssembly.aname

    with_process(&[(BASE, &i)], |process| {
        let module = Module {
            assemblies: Address::new(BASE),
            type_info_definition_table: Address::new(BASE + 0x10),
            version: Version::V2022,
            offsets: IL2CPPOffsets::new(Version::V2022, PointerSize::Bit64).unwrap(),
            pointer_size: PointerSize::Bit32,
        };
        assert!(module.get_default_image(process).is_some());
    });
}
