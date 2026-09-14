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
