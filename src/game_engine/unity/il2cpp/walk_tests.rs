//! Tests pinning the walk's behavior over a hand-laid image of IL2CPP's
//! structures. There is one fixture per type start shape: the Unity 2019.4
//! player keeps the index of its first type inside the image, and the Unity
//! 2022.3 player keeps a pointer to that index. The offsets are the literal
//! numbers of those two players, copied by hand from their PDBs, so the walk
//! is checked against the layout rather than against itself.

use super::{Module, Profile, UnityPointer};
use crate::runtime::mock::{poll_once, with_process};
use crate::{Address, PointerSize, Process};

use core::task::Poll;

use std::vec;
use std::vec::Vec;

const BASE: u64 = 0x20_0000;

fn put(image: &mut [u8], at: u64, bytes: &[u8]) {
    let at = at as usize;
    image[at..at + bytes.len()].copy_from_slice(bytes);
}

// A player as `attach_auto_detect` sees it: `GameAssembly.dll` with a PE
// header and the x64 code that points at both globals, and `UnityPlayer.dll`
// with a PE header and a version resource.
struct Player {
    unity: (u16, u16, u16, u16),
}

const GAME_ASSEMBLY: u64 = 0x1_8000_0000;
const UNITY_PLAYER: u64 = 0x1900_0000;

impl Player {
    fn attach(&self) -> Option<Module> {
        self.with_process(Module::attach_auto_detect)
    }

    fn attach_profile(&self, profile: Profile) -> Option<Module> {
        self.with_process(|process| Module::attach(process, profile))
    }

    fn with_process<T>(&self, f: impl FnOnce(&Process) -> T) -> T {
        let game_assembly = Self::game_assembly();
        let unity_player = Self::unity_player(self.unity);
        crate::runtime::mock::with_modules(
            &[
                (GAME_ASSEMBLY, &game_assembly[..]),
                (UNITY_PLAYER, &unity_player[..]),
            ],
            &[
                ("GameAssembly.dll", GAME_ASSEMBLY, 0x1000),
                ("UnityPlayer.dll", UNITY_PLAYER, 0x1000),
            ],
            f,
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

const MEASURED_2019: (u16, u16, u16, u16) = (2019, 4, 41, 9172);
const MEASURED_2022: (u16, u16, u16, u16) = (2022, 3, 0, 4507);
const MEASURED_6000_5: (u16, u16, u16, u16) = (6000, 5, 10, 54518);

fn measured(unity: (u16, u16, u16, u16), pointer_size: PointerSize) -> Profile {
    super::builds::nearest(unity, pointer_size).unwrap().profile
}

// A game on a measured player attaches with the offsets measured on that
// player.
#[test]
fn attach_auto_detect_uses_a_measured_build() {
    let module = Player {
        unity: MEASURED_6000_5,
    }
    .attach()
    .unwrap();
    assert_eq!(module.assemblies, Address::new(GAME_ASSEMBLY + 0x900));
    assert_eq!(
        module.type_info_definition_table,
        Address::new(GAME_ASSEMBLY + 0x910)
    );
    assert_eq!(module.profile.class.static_fields, 0xA0);
}

#[test]
fn attach_uses_the_explicit_profile_and_checks_its_width() {
    let player = Player {
        unity: MEASURED_6000_5,
    };
    let module = player
        .attach_profile(super::profiles::UNITY_6000_5_10F1_X86_64)
        .unwrap();
    assert_eq!(module.profile.class.static_fields, 0xa0);
    assert!(player
        .attach_profile(super::profiles::UNITY_6000_5_10F1_X86)
        .is_none());
}

// A player nobody measured takes the nearest build.
#[test]
fn attach_auto_detect_takes_the_nearest_build_for_an_unmeasured_player() {
    let module = Player {
        unity: (2021, 3, 5, 1),
    }
    .attach()
    .unwrap();
    let nearest = super::builds::nearest((2021, 3, 11, 23713), PointerSize::Bit64).unwrap();
    assert_eq!(module.profile, nearest.profile);
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

// The scan reads no offsets. The module is the first 0x1000 bytes of the
// image, so the image can hold bytes past the module's end.
fn attach_x86(process: &crate::Process) -> Option<Module> {
    Module::attach_with(
        process,
        (Address::new(BASE), 0x1000),
        measured(MEASURED_6000_5, PointerSize::Bit32),
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
            measured(MEASURED_6000_5, PointerSize::Bit64),
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
            measured(MEASURED_6000_5, PointerSize::Bit64),
        )
        .is_none());
    });
}

// Every measured build names an assembly through its image, at the image's
// own name field.
#[test]
fn assembly_names_resolve_through_the_image() {
    let build = super::builds::nearest(MEASURED_6000_5, PointerSize::Bit64).unwrap();
    let profile = build.profile;
    assert!(profile.assembly.name.is_none());
    let name_at = profile.image.assembly_name.unwrap() as u64;

    let mut i = vec![0; 0x1000];
    let ptr = |i: &mut [u8], at: u64, target: u64| {
        put(i, at, &target.to_le_bytes());
    };
    put(&mut i, 0x800, b"Assembly-CSharp");
    ptr(&mut i, 0x0, BASE + 0x40); // the vector's begin
    ptr(&mut i, 0x8, BASE + 0x48); // and end, one assembly along
    ptr(&mut i, 0x40, BASE + 0x80);
    ptr(&mut i, 0x80, BASE + 0x100); // Il2CppAssembly.image
    ptr(&mut i, 0x100 + name_at, BASE + 0x800); // Il2CppImage.nameNoExt

    with_process(&[(BASE, &i)], |process| {
        let module = Module {
            assemblies: Address::new(BASE),
            type_info_definition_table: Address::new(BASE + 0x10),
            profile,
            pointer_size: PointerSize::Bit64,
        };
        let image = module.get_default_image(process).unwrap();
        assert_eq!(image.image, Address::new(BASE + 0x100));
    });
}

fn ptr(image: &mut [u8], at: u64, target: u64) {
    put(image, at, &target.to_le_bytes());
}

// The target's structures, hand-laid: the assemblies vector, the type info
// definition table sliced by the image's handle, a parent chain reaching a
// UnityEngine class, a static table, and a live object heading with its class.
// The image is laid out by hand from the GameAssembly.pdb of each player, not
// from the player's entry in the table, so a wrong entry fails against this
// image. The Unity 2019.4 player counts types at 0x1C, keeps the index of
// the first type inside the image at 0x18, and counts fields at 0x11C. The
// Unity 2022.3 player counts types at 0x18, keeps a pointer to that index at
// 0x28, and counts fields at 0x124. Both players keep the assembly name in
// the image at 0x8.
fn image(unity: (u16, u16, u16, u16)) -> Vec<u8> {
    let (type_count_at, inline_start, handle_at, field_count_at) = match unity {
        MEASURED_2019 => (0x1C, Some(0x18), None, 0x11C),
        MEASURED_2022 => (0x18, None, Some(0x28), 0x124),
        other => panic!("no hand-laid image for {other:?}"),
    };

    let mut i = vec![0; 0x5000];

    let strings = [
        (0x2000, "mscorlib"),
        (0x2080, "Assembly-CSharp"),
        (0x2100, "GameManager"),
        (0x2180, "Game"),
        (0x2200, "points"),
        (0x2280, "Enemy"),
        (0x2300, "hp"),
        (0x2380, "Boss"),
        (0x2400, "phase"),
        (0x2480, "MonoBehaviour"),
        (0x2500, "UnityEngine"),
        (0x2580, "hidden"),
        (0x2600, "instance"),
        (0x2700, "Outer"),
        (0x2780, "Inner"),
        (0x2800, "spawner"),
        (0x2880, "_items"),
        (0x2900, "_size"),
        (0x2980, "List`1"),
        (0x2A00, "System.Collections.Generic"),
        (0x2A80, "EnemyList"),
        (0x2B00, "ListLookalike"),
    ];
    for (at, text) in strings {
        put(&mut i, at, text.as_bytes());
    }

    // The assemblies vector: begin and end of an array of assembly pointers.
    ptr(&mut i, 0x0, BASE + 0x40);
    ptr(&mut i, 0x8, BASE + 0x50);
    ptr(&mut i, 0x40, BASE + 0x80);
    ptr(&mut i, 0x48, BASE + 0xC0);

    // Il2CppAssembly keeps the image at 0x0. Il2CppImage keeps the name at 0x8.
    ptr(&mut i, 0x80, BASE + 0x140);
    ptr(&mut i, 0x140 + 0x8, BASE + 0x2000);
    ptr(&mut i, 0xC0, BASE + 0x300);
    ptr(&mut i, 0x300 + 0x8, BASE + 0x2080);

    // The default image holds three classes, reached through the index of the
    // first type. The older player keeps that index inside the image, and the
    // newer player keeps a pointer to it.
    put(&mut i, 0x300 + type_count_at, &5_u32.to_le_bytes());
    if let Some(at) = inline_start {
        put(&mut i, 0x300 + at, &5_u32.to_le_bytes());
    }
    if let Some(at) = handle_at {
        ptr(&mut i, 0x300 + at, BASE + 0x400);
        put(&mut i, 0x400, &5_u32.to_le_bytes());
    }

    // The type info definition table global, and the image's slice of it.
    ptr(&mut i, 0x10, BASE + 0x480);
    ptr(&mut i, 0x480 + 8 * 5, BASE + 0x600);
    ptr(&mut i, 0x480 + 8 * 6, BASE + 0x800);
    ptr(&mut i, 0x480 + 8 * 7, BASE + 0xA00);
    ptr(&mut i, 0x480 + 8 * 8, BASE + 0x1200);
    ptr(&mut i, 0x480 + 8 * 9, BASE + 0x1400);

    // Il2CppClass: name 0x10, namespace 0x18, parent 0x58, fields 0x80,
    // static_fields 0xB8, field_count where the lineage keeps it. Field
    // entries stride 0x20 with the name at 0x0 and the offset at 0x18.

    // GameManager, deriving from MonoBehaviour, with a static slot and an
    // instance field.
    let game_manager = 0x600;
    ptr(&mut i, game_manager + 0x10, BASE + 0x2100);
    ptr(&mut i, game_manager + 0x18, BASE + 0x2180);
    ptr(&mut i, game_manager + 0x58, BASE + 0xC00);
    ptr(&mut i, game_manager + 0x80, BASE + 0xE00);
    ptr(&mut i, game_manager + 0xB8, BASE + 0xF40);
    put(&mut i, game_manager + field_count_at, &2_u16.to_le_bytes());
    ptr(&mut i, 0xE00, BASE + 0x2600); // instance
    put(&mut i, 0xE00 + 0x18, &0_i32.to_le_bytes());
    ptr(&mut i, 0xE20, BASE + 0x2200); // points
    put(&mut i, 0xE20 + 0x18, &0x20_i32.to_le_bytes());

    // Enemy with an instance field and a static slot, and Boss deriving from
    // it.
    let enemy = 0x800;
    ptr(&mut i, enemy + 0x10, BASE + 0x2280);
    ptr(&mut i, enemy + 0x18, BASE + 0x2180);
    ptr(&mut i, enemy + 0x80, BASE + 0xE80);
    ptr(&mut i, enemy + 0xB8, BASE + 0xFC0);
    put(&mut i, enemy + field_count_at, &2_u16.to_le_bytes());
    ptr(&mut i, 0xE80, BASE + 0x2300); // hp
    put(&mut i, 0xE80 + 0x18, &0x10_i32.to_le_bytes());
    ptr(&mut i, 0xEA0, BASE + 0x2800); // spawner
    put(&mut i, 0xEA0 + 0x18, &0x8_i32.to_le_bytes());

    let boss = 0xA00;
    ptr(&mut i, boss + 0x10, BASE + 0x2380);
    ptr(&mut i, boss + 0x18, BASE + 0x2180);
    ptr(&mut i, boss + 0x58, BASE + enemy);
    ptr(&mut i, boss + 0x80, BASE + 0xEC0);
    ptr(&mut i, boss + 0xB8, BASE + 0x1000);
    put(&mut i, boss + field_count_at, &1_u16.to_le_bytes());
    ptr(&mut i, 0xEC0, BASE + 0x2400); // phase
    put(&mut i, 0xEC0 + 0x18, &0x18_i32.to_le_bytes());

    // MonoBehaviour in UnityEngine, holding a field the climb must never
    // reach.
    let mono_behaviour = 0xC00;
    ptr(&mut i, mono_behaviour + 0x10, BASE + 0x2480);
    ptr(&mut i, mono_behaviour + 0x18, BASE + 0x2500);
    ptr(&mut i, mono_behaviour + 0x80, BASE + 0xF00);
    put(
        &mut i,
        mono_behaviour + field_count_at,
        &1_u16.to_le_bytes(),
    );
    ptr(&mut i, 0xF00, BASE + 0x2580); // hidden
    put(&mut i, 0xF00 + 0x18, &0x30_i32.to_le_bytes());

    // Outer in Game, enclosing Inner, whose own namespace is empty and whose
    // declaring type points back out.
    let outer = 0x1200;
    ptr(&mut i, outer + 0x10, BASE + 0x2700);
    ptr(&mut i, outer + 0x18, BASE + 0x2180);
    let inner = 0x1400;
    ptr(&mut i, inner + 0x10, BASE + 0x2780);
    ptr(&mut i, inner + 0x18, BASE + 0x27F0);
    ptr(&mut i, inner + 0x50, BASE + outer);

    // GameManager's statics hold the live instance, which heads with its
    // class.
    ptr(&mut i, 0xF40, BASE + 0xF80);
    ptr(&mut i, 0xF80, BASE + game_manager);
    put(&mut i, 0xF80 + 0x20, &888_u32.to_le_bytes());

    // Enemy's statics hold the spawner instance. Boss carries a table of its
    // own, empty at that offset, so only the declaring class's table answers.
    ptr(&mut i, 0xFC0 + 0x8, BASE + 0x1080);

    // A List: its class carries corlib's field names, its live object heads
    // with the class and holds a backing array longer than the live count.
    let list_class = 0x1600;
    ptr(&mut i, list_class + 0x10, BASE + 0x2980);
    ptr(&mut i, list_class + 0x18, BASE + 0x2A00);
    ptr(&mut i, list_class + 0x80, BASE + 0x1780);
    put(&mut i, list_class + field_count_at, &2_u16.to_le_bytes());
    ptr(&mut i, 0x1780, BASE + 0x2880); // _items
    put(&mut i, 0x1780 + 0x18, &0x10_i32.to_le_bytes());
    ptr(&mut i, 0x17A0, BASE + 0x2900); // _size
    put(&mut i, 0x17A0 + 0x18, &0x18_i32.to_le_bytes());

    ptr(&mut i, 0x1800, BASE + list_class); // the list object heads with its class
    ptr(&mut i, 0x1800 + 0x10, BASE + 0x1900);
    put(&mut i, 0x1800 + 0x18, &2_i32.to_le_bytes());
    put(&mut i, 0x1900 + 0x18, &4_u32.to_le_bytes()); // the backing's capacity
    for (index, value) in [11_u32, 22, 100, 100].into_iter().enumerate() {
        put(
            &mut i,
            0x1900 + 0x20 + 4 * index as u64,
            &value.to_le_bytes(),
        );
    }

    ptr(&mut i, 0x18, BASE + 0x1800); // the slot holding the reference

    // A derived list inherits the two fields from List rather than declaring
    // them again.
    let derived_list = 0x3000;
    ptr(&mut i, derived_list + 0x10, BASE + 0x2A80);
    ptr(&mut i, derived_list + 0x18, BASE + 0x2180);
    ptr(&mut i, derived_list + 0x58, BASE + list_class);
    ptr(&mut i, 0x3200, BASE + derived_list);
    ptr(&mut i, 0x3200 + 0x10, BASE + 0x1900);
    put(&mut i, 0x3200 + 0x18, &2_i32.to_le_bytes());
    ptr(&mut i, 0x20, BASE + 0x3200);

    // A lookalike carries fields with the same names but is not the corlib
    // List class and must not be accepted as one.
    let lookalike = 0x3400;
    ptr(&mut i, lookalike + 0x10, BASE + 0x2B00);
    ptr(&mut i, lookalike + 0x18, BASE + 0x2180);
    ptr(&mut i, lookalike + 0x80, BASE + 0x3600);
    put(&mut i, lookalike + field_count_at, &2_u16.to_le_bytes());
    ptr(&mut i, 0x3600, BASE + 0x2880);
    put(&mut i, 0x3600 + 0x18, &0x10_i32.to_le_bytes());
    ptr(&mut i, 0x3620, BASE + 0x2900);
    put(&mut i, 0x3620 + 0x18, &0x18_i32.to_le_bytes());
    ptr(&mut i, 0x3800, BASE + lookalike);
    ptr(&mut i, 0x28, BASE + 0x3800);

    // A reference list shares the class; its backing holds an object
    // address and a null element.
    ptr(&mut i, 0x1A00, BASE + list_class);
    ptr(&mut i, 0x1A00 + 0x10, BASE + 0x1B00);
    put(&mut i, 0x1A00 + 0x18, &2_i32.to_le_bytes());
    put(&mut i, 0x1B00 + 0x18, &2_u64.to_le_bytes());
    ptr(&mut i, 0x1B00 + 0x20, BASE + 0x1080);
    ptr(&mut i, 0x30, BASE + 0x1A00);

    i
}

fn module(unity: (u16, u16, u16, u16)) -> Module {
    Module {
        assemblies: Address::new(BASE),
        type_info_definition_table: Address::new(BASE + 0x10),
        profile: measured(unity, PointerSize::Bit64),
        pointer_size: PointerSize::Bit64,
    }
}

fn on_fixture(unity: (u16, u16, u16, u16), test: impl FnOnce(&Process, &Module)) {
    with_process(&[(BASE, &image(unity))], |process| {
        test(process, &module(unity));
    });
}

#[test]
fn images_resolve_by_name_on_both_type_start_shapes() {
    for unity in [MEASURED_2019, MEASURED_2022] {
        on_fixture(unity, |process, module| {
            assert!(module.get_default_image(process).is_some());
            assert!(module.get_image(process, "mscorlib").is_some());
            assert!(module.get_image(process, "Assembly-DoesNotExist").is_none());
        });
    }
}

#[test]
fn classes_resolve_by_name_and_namespace() {
    for unity in [MEASURED_2019, MEASURED_2022] {
        on_fixture(unity, |process, module| {
            let image = module.get_default_image(process).unwrap();
            assert!(image.get_class(process, module, "GameManager").is_some());
            assert!(image.get_class(process, module, "Game.Boss").is_some());
            assert!(image.get_class(process, module, "Wrong.Boss").is_none());
            assert!(image.get_class(process, module, "Nothing").is_none());
            assert_eq!(image.classes(process, module).count(), 5);
        });
    }
}

#[test]
fn field_offsets_resolve_declared_and_inherited() {
    on_fixture(MEASURED_2022, |process, module| {
        let image = module.get_default_image(process).unwrap();
        let game_manager = image.get_class(process, module, "GameManager").unwrap();
        assert_eq!(
            game_manager.get_field_offset(process, module, "points"),
            Some(0x20),
        );

        let boss = image.get_class(process, module, "Boss").unwrap();
        assert_eq!(boss.get_field_offset(process, module, "phase"), Some(0x18));
        assert_eq!(boss.get_field_offset(process, module, "hp"), Some(0x10));
    });
}

#[test]
fn nested_classes_resolve_by_their_written_name() {
    on_fixture(MEASURED_2022, |process, module| {
        let image = module.get_default_image(process).unwrap();
        assert!(image
            .get_class(process, module, "Game.Outer+Inner")
            .is_some());
        assert!(image
            .get_class(process, module, "Game.Outer+Missing")
            .is_none());
        assert!(image
            .get_class(process, module, "Wrong.Outer+Inner")
            .is_none());
        assert!(image
            .get_class(process, module, "Game.Enemy+Inner")
            .is_none());
    });
}

// The climb stops at UnityEngine's namespace, so an engine field never
// resolves.
#[test]
fn field_climbs_stop_at_the_engine() {
    on_fixture(MEASURED_2022, |process, module| {
        let image = module.get_default_image(process).unwrap();
        let game_manager = image.get_class(process, module, "GameManager").unwrap();
        assert!(game_manager
            .get_field_offset(process, module, "hidden")
            .is_none());
    });
}

#[test]
fn statics_resolve_from_the_class() {
    on_fixture(MEASURED_2022, |process, module| {
        let image = module.get_default_image(process).unwrap();
        let game_manager = image.get_class(process, module, "GameManager").unwrap();
        assert_eq!(
            game_manager.get_static_table(process, module),
            Some(Address::new(BASE + 0xF40)),
        );
    });
}

// A static field found on a parent measures into the parent's own static
// table, not the table of the class the lookup started at.
#[test]
fn static_instances_resolve_through_the_declaring_class() {
    on_fixture(MEASURED_2022, |process, module| {
        let image = module.get_default_image(process).unwrap();
        let boss = image.get_class(process, module, "Boss").unwrap();
        assert_eq!(
            poll_once(boss.wait_get_static_instance(process, module, "spawner")),
            Poll::Ready(Address::new(BASE + 0x1080)),
        );

        let pointer = UnityPointer::<1>::new("Boss", 0, &["spawner"]);
        assert_eq!(
            pointer.deref::<u64>(process, module, &image).unwrap(),
            BASE + 0x1080,
        );
    });
}

// A list's backing array and live count resolve off the corlib List class,
// and the read returns the live count's elements, never the backing capacity's.
#[test]
fn lists_resolve_through_their_own_class() {
    on_fixture(MEASURED_2022, |process, module| {
        let at = Address::new(BASE + 0x18);
        let offsets = module.get_list_offsets(process, at).unwrap();
        let read = module.read_list::<u32, 4>(process, offsets, at).unwrap();
        assert_eq!(read.as_slice(), [11, 22]);
    });
}

#[test]
fn lists_derived_from_corlibs_list_resolve() {
    on_fixture(MEASURED_2022, |process, module| {
        let at = Address::new(BASE + 0x20);
        let offsets = module.get_list_offsets(process, at).unwrap();
        let read = module.read_list::<u32, 4>(process, offsets, at).unwrap();
        assert_eq!(read.as_slice(), [11, 22]);
    });
}

#[test]
fn list_shaped_objects_are_not_lists() {
    on_fixture(MEASURED_2022, |process, module| {
        assert!(module
            .get_list_offsets(process, Address::new(BASE + 0x28))
            .is_none());
    });
}

// A reference list hands back its elements' object addresses, a null
// element preserved at its position.
#[test]
fn reference_lists_resolve_their_element_addresses() {
    on_fixture(MEASURED_2022, |process, module| {
        let at = Address::new(BASE + 0x30);
        let offsets = module.get_list_offsets(process, at).unwrap();
        let read = module
            .read_reference_list::<4>(process, offsets, at)
            .unwrap();
        assert_eq!(
            read.as_slice(),
            [Address::new(BASE + 0x1080), Address::NULL]
        );
    });
}

// The whole pointer path: the static root, the instance behind it, and a field
// resolved against the object's own class read off its head.
#[test]
fn pointers_dereference_through_a_static_root() {
    on_fixture(MEASURED_2022, |process, module| {
        let image = module.get_default_image(process).unwrap();
        let pointer = UnityPointer::<2>::new("GameManager", 0, &["instance", "points"]);
        assert_eq!(pointer.deref::<u32>(process, module, &image).unwrap(), 888);
    });
}

// The public shapes the carve must not change.
#[test]
fn public_types_keep_their_properties() {
    fn is_copy<T: Copy>() {}
    fn double_ended<'a>(
        iter: impl DoubleEndedIterator<Item = super::Class> + 'a,
    ) -> impl DoubleEndedIterator<Item = super::Class> + 'a {
        iter
    }

    is_copy::<super::Image>();
    is_copy::<super::Class>();

    on_fixture(MEASURED_2022, |process, module| {
        let image = module.get_default_image(process).unwrap();
        let _ = double_ended(image.classes(process, module));
    });
}

// A 32 bit target lays the assemblies vector and its pointers at four bytes.
#[test]
fn images_resolve_on_32_bit_targets() {
    let profile = measured(MEASURED_6000_5, PointerSize::Bit32);
    let name_at = profile.image.assembly_name.unwrap() as u64;
    let mut i = vec![0; 0x1000];
    let narrow = |i: &mut [u8], at: u64, target: u64| {
        put(i, at, &(target as u32).to_le_bytes());
    };

    put(&mut i, 0x800, b"Assembly-CSharp");
    narrow(&mut i, 0x0, BASE + 0x40); // the vector's begin
    narrow(&mut i, 0x4, BASE + 0x44); // and end, one assembly along
    narrow(&mut i, 0x40, BASE + 0x80);
    narrow(&mut i, 0x80, BASE + 0x100); // Il2CppAssembly.image
    narrow(&mut i, 0x100 + name_at, BASE + 0x800); // Il2CppImage.nameNoExt

    with_process(&[(BASE, &i)], |process| {
        let module = Module {
            assemblies: Address::new(BASE),
            type_info_definition_table: Address::new(BASE + 0x10),
            profile,
            pointer_size: PointerSize::Bit32,
        };
        assert!(module.get_default_image(process).is_some());
    });
}

// An image says where its first type sits in the type table. Players up to
// 2020.1 keep that index in the image. Later players keep a handle there,
// and the index sits behind it.
#[test]
fn class_walk_reads_the_type_start_where_the_build_keeps_it() {
    use super::TypeStart;

    let walk = |unity, pointer_size: PointerSize| {
        let profile = measured(unity, pointer_size);
        let ps = pointer_size as u64;
        let mut i = vec![0; 0x1000];
        let ptr = |i: &mut [u8], at: u64, target: u64| match pointer_size {
            PointerSize::Bit64 => put(i, at, &target.to_le_bytes()),
            _ => put(i, at, &(target as u32).to_le_bytes()),
        };
        put(&mut i, 0x800, b"Timer");
        ptr(&mut i, 0x10, BASE + 0x200); // the type table
        ptr(&mut i, 0x200 + 2 * ps, BASE + 0x300); // its entry 2
        ptr(&mut i, 0x300 + profile.class.name as u64, BASE + 0x800);
        put(
            &mut i,
            0x100 + profile.image.type_count as u64,
            &1_u32.to_le_bytes(),
        );
        match profile.image.type_start {
            TypeStart::Inline(at) => put(&mut i, 0x100 + at as u64, &2_u32.to_le_bytes()),
            TypeStart::Handle(at) => {
                ptr(&mut i, 0x100 + at as u64, BASE + 0x180);
                put(&mut i, 0x180, &2_u32.to_le_bytes());
            }
        }

        with_process(&[(BASE, &i)], |process| {
            let module = Module {
                assemblies: Address::new(BASE),
                type_info_definition_table: Address::new(BASE + 0x10),
                profile,
                pointer_size,
            };
            let image = super::Image {
                image: Address::new(BASE + 0x100),
            };
            image
                .get_class(process, &module, "Timer")
                .map(|class| class.class)
        })
    };

    for pointer_size in [PointerSize::Bit64, PointerSize::Bit32] {
        let inline = measured((2018, 4, 36, 54151), pointer_size);
        assert!(matches!(inline.image.type_start, TypeStart::Inline(_)));
        assert_eq!(
            walk((2018, 4, 36, 54151), pointer_size),
            Some(Address::new(BASE + 0x300))
        );

        let handle = measured(MEASURED_6000_5, pointer_size);
        assert!(matches!(handle.image.type_start, TypeStart::Handle(_)));
        assert_eq!(
            walk(MEASURED_6000_5, pointer_size),
            Some(Address::new(BASE + 0x300))
        );
    }
}
