//! Tests pinning the walk's behavior over a hand-laid image of mono's
//! structures. The fixture is written at the literal offsets of the Unity
//! 2019.4 x64 runtime, copied by hand, so the walk is checked against the
//! layout rather than against itself.

use super::offsets::{
    AssemblyOffsets, ClassOffsets, FieldInfoOffsets, GenericOffsets, HashTableOffsets,
    ImageOffsets, MonoVTableOffsets,
};
use super::{builds, BinaryFormat, Module, MonoOffsets, UnityPointer, Version};
use crate::file_format::pe::DebugId;
use crate::runtime::mock::{poll_once, with_process};
use crate::{Address, PointerSize, Process};

use core::task::Poll;

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
// reaching a UnityEngine class, static tables reachable through the vtables,
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
        (0x2800, "spawner"),
        (0x2880, "_items"),
        (0x2900, "_size"),
        (0x2980, "_version"),
        (0x2B00, "Inventory"),
        (0x2B80, "items"),
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

    // Enemy, with an instance field and a static slot, and Boss chained
    // behind it in the bucket.
    let enemy = 0xE00;
    ptr(&mut i, enemy + 0x48, BASE + 0x2300);
    ptr(&mut i, enemy + 0x50, BASE + 0x2180);
    put(&mut i, enemy + 0x5C, &1_i32.to_le_bytes());
    ptr(&mut i, enemy + 0x98, BASE + 0x1500);
    ptr(&mut i, enemy + 0xD0, BASE + 0x1620);
    put(&mut i, enemy + 0x100, &2_i32.to_le_bytes());
    ptr(&mut i, enemy + 0x108, BASE + 0x1000);
    ptr(&mut i, 0x1500 + 0x8, BASE + 0x2380); // hp
    put(&mut i, 0x1500 + 0x18, &0x10_i32.to_le_bytes());
    ptr(&mut i, 0x1520 + 0x8, BASE + 0x2800); // spawner
    put(&mut i, 0x1520 + 0x18, &0x8_i32.to_le_bytes());

    // Boss, deriving from Enemy, with one field of its own.
    let boss = 0x1000;
    ptr(&mut i, boss + 0x30, BASE + enemy);
    ptr(&mut i, boss + 0x48, BASE + 0x2400);
    ptr(&mut i, boss + 0x50, BASE + 0x2180);
    put(&mut i, boss + 0x5C, &2_i32.to_le_bytes());
    ptr(&mut i, boss + 0x98, BASE + 0x1540);
    ptr(&mut i, boss + 0xD0, BASE + 0x1650);
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
    ptr(&mut i, inner + 0x108, BASE + 0x2D00);

    // Inventory, a generic instance: its class kind's low bits read 3, its own
    // field count slot holds nothing, and the count lives on the definition
    // reached through the instantiation descriptor. The inflated field array
    // is the instance's own.
    let inventory = 0x2D00;
    ptr(&mut i, inventory + 0x48, BASE + 0x2B00);
    ptr(&mut i, inventory + 0x50, BASE + 0x2180);
    put(&mut i, inventory + 0x2A, &3_u8.to_le_bytes());
    ptr(&mut i, inventory + 0x98, BASE + 0x3400);
    ptr(&mut i, inventory + 0xF0, BASE + 0x3000);
    ptr(&mut i, 0x3000, BASE + 0x3100); // descriptor: container_class at 0x0
    put(&mut i, 0x3100 + 0x100, &1_i32.to_le_bytes()); // the definition's count
    ptr(&mut i, 0x3400 + 0x8, BASE + 0x2B80); // items
    put(&mut i, 0x3400 + 0x18, &0x28_i32.to_le_bytes());

    // GameManager's statics: runtime_info to the domain vtable, whose static
    // slot sits past five method pointers, holding the static table. The
    // table's first slot is the live instance.
    ptr(&mut i, 0x1600 + 0x8, BASE + 0x1700);
    ptr(&mut i, 0x1700 + 0x40 + 8 * 5, BASE + 0x1800);
    ptr(&mut i, 0x1800, BASE + 0x1900);

    // Enemy's statics, holding the spawner instance. Boss carries a table of
    // its own, empty at that offset, so only the declaring class's table
    // answers.
    ptr(&mut i, 0x1620 + 0x8, BASE + 0x1780);
    ptr(&mut i, 0x1780 + 0x40 + 8, BASE + 0x1880);
    ptr(&mut i, 0x1880 + 0x8, BASE + 0x1980);
    ptr(&mut i, 0x1650 + 0x8, BASE + 0x1A40);
    ptr(&mut i, 0x1A40 + 0x40 + 8 * 2, BASE + 0x1AC0);

    // The instance object: its vtable heads it, and the vtable's own head is
    // the class. The points field holds a recognizable value.
    ptr(&mut i, 0x1900, BASE + 0x1A00);
    ptr(&mut i, 0x1A00, BASE + game_manager);
    put(&mut i, 0x1900 + 0x20, &777_u32.to_le_bytes());

    // A List: its class is a generic instance like Inventory, its fields are
    // corlib's own three, and its live object holds a backing array longer
    // than the live count. A second object claims a count past the backing.
    let list_class = 0x3600;
    put(&mut i, list_class + 0x2A, &3_u8.to_le_bytes());
    ptr(&mut i, list_class + 0x98, BASE + 0x3800);
    ptr(&mut i, list_class + 0xF0, BASE + 0x3D00);
    ptr(&mut i, 0x3D00, BASE + 0x3D40); // descriptor: container_class at 0x0
    put(&mut i, 0x3D40 + 0x100, &3_i32.to_le_bytes()); // the definition's count
    ptr(&mut i, 0x3800 + 0x8, BASE + 0x2880); // _items
    put(&mut i, 0x3800 + 0x18, &0x10_i32.to_le_bytes());
    ptr(&mut i, 0x3820 + 0x8, BASE + 0x2900); // _size
    put(&mut i, 0x3820 + 0x18, &0x18_i32.to_le_bytes());
    ptr(&mut i, 0x3840 + 0x8, BASE + 0x2980); // _version
    put(&mut i, 0x3840 + 0x18, &0x1C_i32.to_le_bytes());

    ptr(&mut i, 0x3900, BASE + 0x3950); // the list object heads with its vtable
    ptr(&mut i, 0x3950, BASE + list_class);
    ptr(&mut i, 0x3900 + 0x10, BASE + 0x3A00);
    put(&mut i, 0x3900 + 0x18, &3_i32.to_le_bytes());
    put(&mut i, 0x3A00 + 0x18, &8_u32.to_le_bytes()); // the backing's capacity
    for (index, value) in [5_i32, 6, 7, 100, 100, 100, 100, 100]
        .into_iter()
        .enumerate()
    {
        put(
            &mut i,
            0x3A00 + 0x20 + 4 * index as u64,
            &value.to_le_bytes(),
        );
    }

    ptr(&mut i, 0x3B00, BASE + 0x3950); // the torn list shares the class
    ptr(&mut i, 0x3B00 + 0x10, BASE + 0x3A00);
    put(&mut i, 0x3B00 + 0x18, &99_i32.to_le_bytes());

    ptr(&mut i, 0x3C00, BASE + 0x3C50); // an Inventory object, not a list
    ptr(&mut i, 0x3C50, BASE + 0x2D00);

    // The slots holding the three references.
    ptr(&mut i, 0x3F00, BASE + 0x3900);
    ptr(&mut i, 0x3F08, BASE + 0x3B00);
    ptr(&mut i, 0x3F10, BASE + 0x3C00);

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
        assert_eq!(image.classes(process, module).count(), 6);
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
            class_kind: None,
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
        generic: GenericOffsets {
            generic_class: None,
            container_class: None,
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

        let inventory = image.get_class(process, module, "Inventory").unwrap();
        assert!(inventory
            .get_field_offset(process, module, "items")
            .is_none());
    });
}

// A generic instance declares no count of its own; the definition it was made
// from holds it, and the inflated fields are the instance's.
#[test]
fn generic_field_counts_resolve_through_the_definition() {
    on_fixture(era(), |process, module| {
        let image = module.get_default_image(process).unwrap();
        let inventory = image.get_class(process, module, "Inventory").unwrap();
        assert_eq!(
            inventory.get_field_offset(process, module, "items"),
            Some(0x28),
        );
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

        let outer = image.get_class(process, module, "Game.Outer").unwrap();
        assert!(outer.get_static_table(process, module).is_none());
    });
}

// A static field found on a parent measures into the parent's own static
// table, not the table of the class the lookup started at.
#[test]
fn static_instances_resolve_through_the_declaring_class() {
    on_fixture(era(), |process, module| {
        let image = module.get_default_image(process).unwrap();
        let boss = image.get_class(process, module, "Boss").unwrap();
        assert_eq!(
            poll_once(boss.wait_get_static_instance(process, module, "spawner")),
            Poll::Ready(Address::new(BASE + 0x1980)),
        );

        let pointer = UnityPointer::<1>::new("Boss", 0, &["spawner"]);
        assert_eq!(
            pointer.deref::<u64>(process, module, &image).unwrap(),
            BASE + 0x1980,
        );
    });
}

// A list's backing array and live count resolve off the list object's own
// class, and the read returns the live count's elements, never the backing
// capacity's, which the buffer size does not judge.
#[test]
fn lists_resolve_through_their_own_class() {
    on_fixture(era(), |process, module| {
        let at = Address::new(BASE + 0x3F00);
        let offsets = module.get_list_offsets(process, at).unwrap();
        let read = module.read_list::<i32, 4>(process, offsets, at).unwrap();
        assert_eq!(read.as_slice(), [5, 6, 7]);
    });
}

// A count past the backing array's own length is a torn resize, not a long
// list.
#[test]
fn list_counts_past_their_backing_refuse() {
    on_fixture(era(), |process, module| {
        let at = Address::new(BASE + 0x3F08);
        let offsets = module.get_list_offsets(process, at).unwrap();
        assert!(module.read_list::<i32, 128>(process, offsets, at).is_err());
    });
}

// An object whose class does not carry corlib's names is not a list, and
// misses cleanly rather than answering with whatever offsets exist.
#[test]
fn objects_that_are_not_lists_answer_nothing() {
    on_fixture(era(), |process, module| {
        assert!(module
            .get_list_offsets(process, Address::new(BASE + 0x3F10))
            .is_none());
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
