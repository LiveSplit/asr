//! Tests pinning the walk's behavior over a hand-laid image of mono's
//! structures. The fixture is written at the literal offsets of the Unity
//! 2019.4 x64 runtime, copied by hand, so the walk is checked against the
//! layout rather than against itself.

use super::offsets::{
    AssemblyOffsets, ClassOffsets, FieldInfoOffsets, HashTableOffsets, ImageOffsets,
    MonoVTableOffsets,
};
use super::{builds, BinaryFormat, Module, MonoOffsets, UnityPointer, Version};
use crate::file_format::pe::DebugId;
use crate::runtime::mock::with_process;
use crate::{Address, PointerSize, Process};

use std::vec;
use std::vec::Vec;

const BASE: u64 = 0x10_0000;

fn put(image: &mut [u8], at: u64, bytes: &[u8]) {
    let at = at as usize;
    image[at..at + bytes.len()].copy_from_slice(bytes);
}

fn ptr(image: &mut [u8], at: u64, target: u64) {
    put(image, at, &target.to_le_bytes());
}

// The target's structures, hand-laid. Two assemblies whose GList the walk
// follows, a class cache of two buckets with one chained class, a parent chain
// reaching a UnityEngine class, a static table reachable through the vtable,
// and a live object carrying its class through its vtable.
fn image() -> Vec<u8> {
    let mut i = vec![0; 0x4000];

    // Strings, each 0x80 apart so a 128-byte name read stays in bounds.
    let strings = [
        (0x2000, "mscorlib"),
        (0x2080, "Assembly-CSharp"),
        (0x2100, "GameManager"),
        (0x2180, "Game"),
        (0x2200, "points"),
        (0x2280, "<Health>k__BackingField"),
        (0x2300, "Enemy"),
        (0x2380, "hp"),
        (0x2400, "Boss"),
        (0x2480, "phase"),
        (0x2500, "MonoBehaviour"),
        (0x2580, "UnityEngine"),
        (0x2600, "hidden"),
        (0x2680, "instance"),
        (0x2700, "Outer"),
        (0x2780, "Inner"),
    ];
    for (at, text) in strings {
        put(&mut i, at, text.as_bytes());
    }

    // The loaded-assemblies global and its GList: mscorlib first, then the
    // default image.
    ptr(&mut i, 0x0, BASE + 0x10);
    ptr(&mut i, 0x10, BASE + 0x40); // node 1: data
    ptr(&mut i, 0x18, BASE + 0x20); // node 1: next
    ptr(&mut i, 0x20, BASE + 0xC0); // node 2: data
    ptr(&mut i, 0x28, 0); // node 2: next

    // MonoAssembly: the name at 0x10 (the aname route reads the pointer that
    // heads MonoAssemblyName), the image at 0x60.
    ptr(&mut i, 0x40 + 0x10, BASE + 0x2000);
    ptr(&mut i, 0x40 + 0x60, BASE + 0x140);
    ptr(&mut i, 0xC0 + 0x10, BASE + 0x2080);
    ptr(&mut i, 0xC0 + 0x60, BASE + 0x640);

    // MonoImage: assembly_name at 0x28, class_cache at 0x4C0 with the hash
    // table's size at +0x18 and bucket array at +0x20. mscorlib's image stays
    // empty; the default image holds two buckets and three classes.
    ptr(&mut i, 0x140 + 0x28, BASE + 0x2000);
    ptr(&mut i, 0x640 + 0x28, BASE + 0x2080);
    put(&mut i, 0x640 + 0x4C0 + 0x18, &2_i32.to_le_bytes());
    ptr(&mut i, 0x640 + 0x4C0 + 0x20, BASE + 0xB40);
    ptr(&mut i, 0xB40, BASE + 0xC00); // bucket 0: GameManager
    ptr(&mut i, 0xB48, BASE + 0xE00); // bucket 1: Enemy, chaining to Boss (kept)

    // MonoClass: parent 0x30, name 0x48, namespace 0x50, vtable_size 0x5C,
    // fields 0x98, runtime_info 0xD0, field_count 0x100, next_class_cache
    // 0x108. Field entries stride 0x20 with the name at 0x8 and the offset at
    // 0x18.

    // GameManager, deriving from MonoBehaviour, with an instance field, a
    // backing field, and a static slot at the head of its field list.
    let game_manager = 0xC00;
    ptr(&mut i, game_manager + 0x30, BASE + 0x1200);
    ptr(&mut i, game_manager + 0x48, BASE + 0x2100);
    ptr(&mut i, game_manager + 0x50, BASE + 0x2180);
    put(&mut i, game_manager + 0x5C, &5_i32.to_le_bytes());
    ptr(&mut i, game_manager + 0x98, BASE + 0x1400);
    ptr(&mut i, game_manager + 0xD0, BASE + 0x1600);
    put(&mut i, game_manager + 0x100, &3_i32.to_le_bytes());
    ptr(&mut i, game_manager + 0x108, BASE + 0x1B00);
    ptr(&mut i, 0x1400 + 0x8, BASE + 0x2680); // instance
    put(&mut i, 0x1400 + 0x18, &0_i32.to_le_bytes());
    ptr(&mut i, 0x1420 + 0x8, BASE + 0x2200); // points
    put(&mut i, 0x1420 + 0x18, &0x20_i32.to_le_bytes());
    ptr(&mut i, 0x1440 + 0x8, BASE + 0x2280); // <Health>k__BackingField
    put(&mut i, 0x1440 + 0x18, &0x24_i32.to_le_bytes());

    // Enemy, with one field and Boss chained behind it in the bucket.
    let enemy = 0xE00;
    ptr(&mut i, enemy + 0x48, BASE + 0x2300);
    ptr(&mut i, enemy + 0x50, BASE + 0x2180);
    ptr(&mut i, enemy + 0x98, BASE + 0x1500);
    put(&mut i, enemy + 0x100, &1_i32.to_le_bytes());
    ptr(&mut i, enemy + 0x108, BASE + 0x1000);
    ptr(&mut i, 0x1500 + 0x8, BASE + 0x2380); // hp
    put(&mut i, 0x1500 + 0x18, &0x10_i32.to_le_bytes());

    // Boss, deriving from Enemy, with one field of its own.
    let boss = 0x1000;
    ptr(&mut i, boss + 0x30, BASE + enemy);
    ptr(&mut i, boss + 0x48, BASE + 0x2400);
    ptr(&mut i, boss + 0x50, BASE + 0x2180);
    ptr(&mut i, boss + 0x98, BASE + 0x1540);
    put(&mut i, boss + 0x100, &1_i32.to_le_bytes());
    ptr(&mut i, 0x1540 + 0x8, BASE + 0x2480); // phase
    put(&mut i, 0x1540 + 0x18, &0x18_i32.to_le_bytes());

    // MonoBehaviour in UnityEngine, holding a field the climb must never
    // reach.
    let mono_behaviour = 0x1200;
    ptr(&mut i, mono_behaviour + 0x48, BASE + 0x2500);
    ptr(&mut i, mono_behaviour + 0x50, BASE + 0x2580);
    ptr(&mut i, mono_behaviour + 0x98, BASE + 0x1580);
    put(&mut i, mono_behaviour + 0x100, &1_i32.to_le_bytes());
    ptr(&mut i, 0x1580 + 0x8, BASE + 0x2600); // hidden
    put(&mut i, 0x1580 + 0x18, &0x30_i32.to_le_bytes());

    // Outer in Game, enclosing Inner, whose own namespace is empty and whose
    // nested_in points back out.
    let outer = 0x1B00;
    ptr(&mut i, outer + 0x48, BASE + 0x2700);
    ptr(&mut i, outer + 0x50, BASE + 0x2180);
    ptr(&mut i, outer + 0x108, BASE + 0x1D00);
    let inner = 0x1D00;
    ptr(&mut i, inner + 0x48, BASE + 0x2780);
    ptr(&mut i, inner + 0x50, BASE + 0x27F0);
    ptr(&mut i, inner + 0x38, BASE + outer);

    // GameManager's statics: runtime_info to the domain vtable, whose static
    // slot sits past five method pointers, holding the static table. The
    // table's first slot is the live instance.
    ptr(&mut i, 0x1600 + 0x8, BASE + 0x1700);
    ptr(&mut i, 0x1700 + 0x40 + 8 * 5, BASE + 0x1800);
    ptr(&mut i, 0x1800, BASE + 0x1900);

    // The instance object: its vtable heads it, and the vtable's own head is
    // the class. The points field holds a recognizable value.
    ptr(&mut i, 0x1900, BASE + 0x1A00);
    ptr(&mut i, 0x1A00, BASE + game_manager);
    put(&mut i, 0x1900 + 0x20, &777_u32.to_le_bytes());

    i
}

fn module(offsets: &'static MonoOffsets) -> Module {
    Module {
        assemblies: Address::new(BASE),
        version: Version::V2,
        offsets,
        pointer_size: PointerSize::Bit64,
    }
}

fn era() -> &'static MonoOffsets {
    MonoOffsets::new(Version::V2, PointerSize::Bit64, BinaryFormat::PE).unwrap()
}

fn measured() -> &'static MonoOffsets {
    // The 2019.4 x64 build the fixture is laid at.
    let stored = [
        0xC7, 0xAA, 0x10, 0x77, 0x5A, 0x31, 0x30, 0x4D, 0xA7, 0x7A, 0x08, 0x07, 0x29, 0x69, 0x66,
        0xF6,
    ];
    &builds::find(&DebugId {
        guid: stored,
        age: 1,
    })
    .unwrap()
    .offsets
}

fn on_fixture(offsets: &'static MonoOffsets, test: impl FnOnce(&Process, &Module)) {
    with_process(&[(BASE, &image())], |process| {
        test(process, &module(offsets));
    });
}

#[test]
fn images_resolve_by_name_through_both_routes() {
    for offsets in [era(), measured()] {
        on_fixture(offsets, |process, module| {
            assert!(module.get_default_image(process).is_some());
            assert!(module.get_image(process, "mscorlib").is_some());
            assert!(module.get_image(process, "Assembly-DoesNotExist").is_none());
        });
    }
}

#[test]
fn classes_resolve_by_name_and_namespace() {
    on_fixture(era(), |process, module| {
        let image = module.get_default_image(process).unwrap();
        assert!(image.get_class(process, module, "GameManager").is_some());
        assert!(image.get_class(process, module, "Game.Boss").is_some());
        assert!(image.get_class(process, module, "Wrong.Boss").is_none());
        assert!(image.get_class(process, module, "Nothing").is_none());
        assert_eq!(image.classes(process, module).count(), 5);
    });
}

#[test]
fn field_offsets_resolve_declared_inherited_and_backing() {
    on_fixture(era(), |process, module| {
        let image = module.get_default_image(process).unwrap();
        let game_manager = image.get_class(process, module, "GameManager").unwrap();
        assert_eq!(
            game_manager.get_field_offset(process, module, "points"),
            Some(0x20),
        );
        assert_eq!(
            game_manager.get_field_offset(process, module, "Health"),
            Some(0x24),
        );

        let boss = image.get_class(process, module, "Boss").unwrap();
        assert_eq!(boss.get_field_offset(process, module, "phase"), Some(0x18));
        assert_eq!(boss.get_field_offset(process, module, "hp"), Some(0x10));
    });
}

#[test]
fn nested_classes_resolve_by_their_written_name() {
    on_fixture(era(), |process, module| {
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

// Offsets that never measured where a class keeps its enclosing class must
// miss cleanly rather than answer with whichever class carries the leaf name.
#[test]
fn nested_lookups_without_a_measured_offset_answer_nothing() {
    static UNMEASURED: MonoOffsets = MonoOffsets {
        assembly: AssemblyOffsets {
            aname: Some(0x10),
            image: 0x60,
        },
        image: ImageOffsets {
            assembly_name: None,
            class_cache: 0x4C0,
        },
        hash_table: HashTableOffsets {
            size: 0x18,
            table: 0x20,
        },
        class: ClassOffsets {
            parent: 0x30,
            nested_in: None,
            name: 0x48,
            namespace: 0x50,
            vtable_size: 0x5C,
            fields: 0x98,
            runtime_info: 0xD0,
            field_count: 0x100,
            next_class_cache: 0x108,
        },
        field: FieldInfoOffsets {
            name: 0x8,
            offset: 0x18,
            alignment: 0x20,
        },
        v_table: MonoVTableOffsets { vtable: 0x40 },
    };

    on_fixture(&UNMEASURED, |process, module| {
        let image = module.get_default_image(process).unwrap();
        assert!(image
            .get_class(process, module, "Game.Outer+Inner")
            .is_none());
        assert!(image.get_class(process, module, "GameManager").is_some());
    });
}

// The climb stops at UnityEngine's namespace, so an engine field never
// resolves.
#[test]
fn field_climbs_stop_at_the_engine() {
    on_fixture(era(), |process, module| {
        let image = module.get_default_image(process).unwrap();
        let game_manager = image.get_class(process, module, "GameManager").unwrap();
        assert!(game_manager
            .get_field_offset(process, module, "hidden")
            .is_none());
    });
}

#[test]
fn statics_resolve_through_the_vtable() {
    on_fixture(era(), |process, module| {
        let image = module.get_default_image(process).unwrap();
        let game_manager = image.get_class(process, module, "GameManager").unwrap();
        assert_eq!(
            game_manager.get_static_table(process, module),
            Some(Address::new(BASE + 0x1800)),
        );

        let boss = image.get_class(process, module, "Boss").unwrap();
        assert!(boss.get_static_table(process, module).is_none());
    });
}

// The whole pointer path: the static root, the instance behind it, and a field
// resolved against the object's own class read through its vtable.
#[test]
fn pointers_dereference_through_a_static_root() {
    on_fixture(era(), |process, module| {
        let image = module.get_default_image(process).unwrap();
        let pointer = UnityPointer::<2>::new("GameManager", 0, &["instance", "points"]);
        assert_eq!(pointer.deref::<u32>(process, module, &image).unwrap(), 777,);
    });
}

// The public shapes the carve must not change.
#[test]
fn public_types_keep_their_properties() {
    fn is_copy<T: Copy>() {}
    fn fused<'a>(
        iter: impl core::iter::FusedIterator<Item = super::Class> + 'a,
    ) -> impl core::iter::FusedIterator<Item = super::Class> + 'a {
        iter
    }

    is_copy::<super::Image>();
    is_copy::<super::Class>();

    on_fixture(era(), |process, module| {
        let image = module.get_default_image(process).unwrap();
        let _ = fused(image.classes(process, module));
    });
}
