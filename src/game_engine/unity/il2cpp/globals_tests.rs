//! Tests finding the two globals through the functions IL2CPP exports. Each
//! test builds a small `GameAssembly.dll` with an export table and the code
//! shape one kind of build has.

use super::globals;
use crate::{
    runtime::mock::{put_exports, with_process},
    Address, PointerSize,
};

use std::vec;
use std::vec::Vec;

const BASE: u64 = 0x1000_0000;
const SIZE: usize = 0x4000;

/// Builds a PE image with the given exports and code. Each piece of code is
/// an offset and its bytes.
fn image(pointer_size: PointerSize, exports: &[(&str, u32)], code: &[(u32, &[u8])]) -> Vec<u8> {
    let mut image = vec![0; SIZE];
    put_exports(&mut image, pointer_size, exports);
    for (at, bytes) in code {
        let at = *at as usize;
        image[at..at + bytes.len()].copy_from_slice(bytes);
    }
    image
}

/// The four bytes of a displacement from `next` to `target`.
const fn displacement(next: u32, target: u32) -> [u8; 4] {
    (target.wrapping_sub(next) as i32).to_le_bytes()
}

/// The four bytes of the absolute address of `target`.
const fn absolute(target: u32) -> [u8; 4] {
    (BASE as u32 + target).to_le_bytes()
}

fn assemblies(pointer_size: PointerSize, code: &[(u32, &[u8])]) -> Option<Address> {
    let image = image(
        pointer_size,
        &[("il2cpp_domain_get_assemblies", 0x1000)],
        code,
    );
    with_process(&[(BASE, &image)], |process| {
        globals::assemblies(process, (Address::new(BASE), SIZE as u64), pointer_size)
    })
}

#[test]
fn finds_s_assemblies_through_the_x64_release_getter() {
    // sub rsp, 0x28; call getter. The getter is lea rax, [s_Assemblies]; ret.
    let call = [
        &[0x48, 0x83, 0xEC, 0x28, 0xE8][..],
        &displacement(0x1009, 0x1100),
    ]
    .concat();
    let getter = [
        &[0x48, 0x8D, 0x05][..],
        &displacement(0x1107, 0x3000),
        &[0xC3],
    ]
    .concat();
    assert_eq!(
        assemblies(PointerSize::Bit64, &[(0x1000, &call), (0x1100, &getter)]),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn finds_s_assemblies_in_the_x64_master_export() {
    // mov rax, [s_Assemblies + 8]; sub rax, [s_Assemblies]; sar rax, 3;
    // mov [rdx], rax; mov rax, [s_Assemblies]; ret
    let code = [
        &[0x48, 0x8B, 0x05][..],
        &displacement(0x1007, 0x3008),
        &[0x48, 0x2B, 0x05],
        &displacement(0x100E, 0x3000),
        &[0x48, 0xC1, 0xF8, 0x03, 0x48, 0x89, 0x02, 0x48, 0x8B, 0x05],
        &displacement(0x101C, 0x3000),
        &[0xC3],
    ]
    .concat();
    assert_eq!(
        assemblies(PointerSize::Bit64, &[(0x1000, &code)]),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn finds_s_assemblies_through_the_x86_release_getter() {
    // push ebp; mov ebp, esp; call getter. The getter is mov eax, s_Assemblies; ret.
    let call = [&[0x55, 0x8B, 0xEC, 0xE8][..], &displacement(0x1008, 0x1100)].concat();
    let getter = [&[0xB8][..], &absolute(0x3000), &[0xC3]].concat();
    assert_eq!(
        assemblies(PointerSize::Bit32, &[(0x1000, &call), (0x1100, &getter)]),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn finds_s_assemblies_in_the_x86_master_export() {
    // push ebp; mov ebp, esp; mov ecx, [s_Assemblies + 4]; mov eax, [ebp + 0xC];
    // sub ecx, [s_Assemblies]; sar ecx, 2; mov [eax], ecx;
    // mov eax, [s_Assemblies]; pop ebp; ret
    let code = [
        &[0x55, 0x8B, 0xEC, 0x8B, 0x0D][..],
        &absolute(0x3004),
        &[0x8B, 0x45, 0x0C, 0x2B, 0x0D],
        &absolute(0x3000),
        &[0xC1, 0xF9, 0x02, 0x89, 0x08, 0xA1],
        &absolute(0x3000),
        &[0x5D, 0xC3],
    ]
    .concat();
    assert_eq!(
        assemblies(PointerSize::Bit32, &[(0x1000, &code)]),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn takes_the_getter_over_a_load_in_the_next_function() {
    // sub rsp, 0x28; call getter; add rsp, 0x28; ret. The next function
    // starts at 0x1010 and ends with mov rax, [other]; ret.
    let call = [
        &[0x48, 0x83, 0xEC, 0x28, 0xE8][..],
        &displacement(0x1009, 0x1100),
        &[0x48, 0x83, 0xC4, 0x28, 0xC3],
    ]
    .concat();
    let next = [
        &[0x48, 0x8B, 0x05][..],
        &displacement(0x1017, 0x3800),
        &[0xC3],
    ]
    .concat();
    let getter = [
        &[0x48, 0x8D, 0x05][..],
        &displacement(0x1107, 0x3000),
        &[0xC3],
    ]
    .concat();
    assert_eq!(
        assemblies(
            PointerSize::Bit64,
            &[(0x1000, &call), (0x1010, &next), (0x1100, &getter)]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn skips_a_call_that_does_not_go_to_the_getter() {
    // call other; call getter. The other function starts with push rbx.
    let calls = [
        &[0xE8][..],
        &displacement(0x1005, 0x1200),
        &[0xE8],
        &displacement(0x100A, 0x1100),
    ]
    .concat();
    let getter = [
        &[0x48, 0x8D, 0x05][..],
        &displacement(0x1107, 0x3000),
        &[0xC3],
    ]
    .concat();
    assert_eq!(
        assemblies(
            PointerSize::Bit64,
            &[(0x1000, &calls), (0x1100, &getter), (0x1200, &[0x40, 0x53])]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn finds_no_s_assemblies_without_the_export() {
    let image = image(PointerSize::Bit64, &[], &[]);
    let found = with_process(&[(BASE, &image)], |process| {
        globals::assemblies(
            process,
            (Address::new(BASE), SIZE as u64),
            PointerSize::Bit64,
        )
    });
    assert_eq!(found, None);
}

fn table(pointer_size: PointerSize, export: &str, code: &[(u32, &[u8])]) -> Option<Address> {
    let image = image(pointer_size, &[(export, 0x1000)], code);
    with_process(&[(BASE, &image)], |process| {
        globals::type_info_definition_table(
            process,
            (Address::new(BASE), SIZE as u64),
            pointer_size,
        )
    })
}

/// A `jmp` at `at` to `target`.
fn jmp(at: u32, target: u32) -> Vec<u8> {
    [&[0xE9][..], &displacement(at + 5, target)].concat()
}

#[test]
fn finds_the_x64_table_two_jumps_into_il2cpp_image_get_class() {
    // mov rax, [s_TypeInfoDefinitionTable]; cmp qword ptr [rdi + rax], 0
    let accessor = [
        &[0x48, 0x8B, 0x05][..],
        &displacement(0x1207, 0x3000),
        &[0x48, 0x83, 0x3C, 0x07, 0x00],
    ]
    .concat();
    let code = [
        (0x1000, jmp(0x1000, 0x1100)),
        (0x1100, jmp(0x1100, 0x1200)),
        (0x1200, accessor),
    ];
    let code: Vec<(u32, &[u8])> = code.iter().map(|(at, bytes)| (*at, &bytes[..])).collect();
    assert_eq!(
        table(PointerSize::Bit64, "il2cpp_image_get_class", &code),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn finds_the_x64_table_through_a_lea_of_the_entry() {
    // call accessor. The accessor is mov rax, [s_TypeInfoDefinitionTable];
    // lea rsi, [rax + rdi*8].
    let call = [&[0xE8][..], &displacement(0x1005, 0x1100)].concat();
    let accessor = [
        &[0x48, 0x8B, 0x05][..],
        &displacement(0x1107, 0x3000),
        &[0x48, 0x8D, 0x34, 0xF8],
    ]
    .concat();
    assert_eq!(
        table(
            PointerSize::Bit64,
            "il2cpp_image_get_class",
            &[(0x1000, &call), (0x1100, &accessor)]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn finds_the_x86_table_through_either_indexed_read() {
    // mov eax, [s_TypeInfoDefinitionTable]; cmp dword ptr [eax + esi*4], 0
    let compared = [&[0xA1][..], &absolute(0x3000), &[0x83, 0x3C, 0xB0, 0x00]].concat();
    assert_eq!(
        table(
            PointerSize::Bit32,
            "il2cpp_image_get_class",
            &[(0x1000, &compared)]
        ),
        Some(Address::new(BASE + 0x3000))
    );
    // mov eax, [s_TypeInfoDefinitionTable]; mov esi, dword ptr [eax + edi*4]
    let loaded = [&[0xA1][..], &absolute(0x3000), &[0x8B, 0x34, 0xB8]].concat();
    assert_eq!(
        table(
            PointerSize::Bit32,
            "il2cpp_image_get_class",
            &[(0x1000, &loaded)]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn starts_at_il2cpp_type_get_class_or_element_class_without_il2cpp_image_get_class() {
    let accessor = [
        &[0x48, 0x8B, 0x05][..],
        &displacement(0x1007, 0x3000),
        &[0x48, 0x83, 0x3C, 0x03, 0x00],
    ]
    .concat();
    assert_eq!(
        table(
            PointerSize::Bit64,
            "il2cpp_type_get_class_or_element_class",
            &[(0x1000, &accessor)]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn skips_a_global_that_is_not_indexed() {
    // mov rax, [other]; test rax, rax; then a jmp to the accessor.
    let decoy = [
        &[0x48, 0x8B, 0x05][..],
        &displacement(0x1007, 0x3800),
        &[0x48, 0x85, 0xC0],
        &jmp(0x100A, 0x1100),
    ]
    .concat();
    let accessor = [
        &[0x48, 0x8B, 0x05][..],
        &displacement(0x1107, 0x3000),
        &[0x48, 0x83, 0x3C, 0x07, 0x00],
    ]
    .concat();
    assert_eq!(
        table(
            PointerSize::Bit64,
            "il2cpp_image_get_class",
            &[(0x1000, &decoy), (0x1100, &accessor)]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn gives_up_past_four_jumps() {
    let mut code: Vec<(u32, Vec<u8>)> = (0..5)
        .map(|hop| {
            (
                0x1000 + hop * 0x100,
                jmp(0x1000 + hop * 0x100, 0x1100 + hop * 0x100),
            )
        })
        .collect();
    code.push((
        0x1500,
        [
            &[0x48, 0x8B, 0x05][..],
            &displacement(0x1507, 0x3000),
            &[0x48, 0x83, 0x3C, 0x07, 0x00],
        ]
        .concat(),
    ));
    let code: Vec<(u32, &[u8])> = code.iter().map(|(at, bytes)| (*at, &bytes[..])).collect();
    assert_eq!(
        table(PointerSize::Bit64, "il2cpp_image_get_class", &code),
        None
    );
}

/// The table accessor at `at`: mov rax, [s_TypeInfoDefinitionTable];
/// cmp qword ptr [rdi + rax], 0, with the table at 0x3000.
fn accessor_x64(at: u32) -> Vec<u8> {
    [
        &[0x48, 0x8B, 0x05][..],
        &displacement(at + 7, 0x3000),
        &[0x48, 0x83, 0x3C, 0x07, 0x00],
    ]
    .concat()
}

#[test]
fn finds_the_table_four_jumps_in() {
    let mut code: Vec<(u32, Vec<u8>)> = (0..4)
        .map(|hop| {
            (
                0x1000 + hop * 0x100,
                jmp(0x1000 + hop * 0x100, 0x1100 + hop * 0x100),
            )
        })
        .collect();
    code.push((0x1400, accessor_x64(0x1400)));
    let code: Vec<(u32, &[u8])> = code.iter().map(|(at, bytes)| (*at, &bytes[..])).collect();
    assert_eq!(
        table(PointerSize::Bit64, "il2cpp_image_get_class", &code),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn searches_a_level_of_more_than_256_functions() {
    // The export calls 10 functions, and each of those calls 30 more. The
    // accessor is the last of the 300.
    let call = |at: u32, target: u32| [&[0xE8][..], &displacement(at + 5, target)].concat();
    let mut code: Vec<(u32, Vec<u8>)> = vec![(
        0x1000,
        (0..10)
            .flat_map(|i| call(0x1000 + i * 5, 0x1100 + i * 0x100))
            .collect(),
    )];
    for i in 0..10 {
        let function = 0x1100 + i * 0x100;
        let calls = (0..30)
            .flat_map(|j| {
                let target = if i == 9 && j == 29 {
                    0x3800
                } else {
                    0x2000 + (i * 30 + j) * 8
                };
                call(function + j * 5, target)
            })
            .collect();
        code.push((function, calls));
    }
    code.push((0x3800, accessor_x64(0x3800)));
    let code: Vec<(u32, &[u8])> = code.iter().map(|(at, bytes)| (*at, &bytes[..])).collect();
    assert_eq!(
        table(PointerSize::Bit64, "il2cpp_image_get_class", &code),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn stops_reading_at_the_module_end() {
    // The export sits in the module's last 0x10 bytes, and the accessor
    // shape comes right after the module, in memory that isn't the module's.
    let image = image(
        PointerSize::Bit64,
        &[("il2cpp_image_get_class", 0x10F0)],
        &[(0x1100, &accessor_x64(0x1100))],
    );
    let found = with_process(&[(BASE, &image)], |process| {
        globals::type_info_definition_table(
            process,
            (Address::new(BASE), 0x1100),
            PointerSize::Bit64,
        )
    });
    assert_eq!(found, None);
}

#[test]
fn prefers_il2cpp_image_get_class_when_both_are_exported() {
    let other = [
        &[0x48, 0x8B, 0x05][..],
        &displacement(0x1107, 0x3800),
        &[0x48, 0x83, 0x3C, 0x07, 0x00],
    ]
    .concat();
    let image = image(
        PointerSize::Bit64,
        &[
            ("il2cpp_type_get_class_or_element_class", 0x1100),
            ("il2cpp_image_get_class", 0x1000),
        ],
        &[(0x1000, &accessor_x64(0x1000)), (0x1100, &other)],
    );
    let found = with_process(&[(BASE, &image)], |process| {
        globals::type_info_definition_table(
            process,
            (Address::new(BASE), SIZE as u64),
            PointerSize::Bit64,
        )
    });
    assert_eq!(found, Some(Address::new(BASE + 0x3000)));
}

#[test]
fn master_export_does_not_follow_a_call_in_the_next_function() {
    let master = [
        &[0x48, 0x8B, 0x05][..],
        &displacement(0x1007, 0x3000),
        &[0xC3],
    ]
    .concat();
    let next = [&[0xE8][..], &displacement(0x1015, 0x1100), &[0xC3]].concat();
    let other_getter = [
        &[0x48, 0x8D, 0x05][..],
        &displacement(0x1107, 0x3800),
        &[0xC3],
    ]
    .concat();
    assert_eq!(
        assemblies(
            PointerSize::Bit64,
            &[(0x1000, &master), (0x1010, &next), (0x1100, &other_getter)]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn type_export_does_not_match_an_indexed_load_in_the_next_function() {
    let decoy = [
        &[0x48, 0x8B, 0x05][..],
        &displacement(0x1017, 0x3800),
        &[0x48, 0x83, 0x3C, 0x07, 0x00, 0xC3],
    ]
    .concat();
    assert_eq!(
        table(
            PointerSize::Bit64,
            "il2cpp_image_get_class",
            &[
                (0x1000, &jmp(0x1000, 0x1200)),
                (0x1010, &decoy),
                (0x1200, &accessor_x64(0x1200))
            ]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn walks_each_part_of_a_function_once() {
    // The export jumps to the accessor when a check fails. After that come 40
    // short branches that each land on the next instruction.
    let mut export = [&[0x0F, 0x85][..], &displacement(0x1006, 0x2000)].concat();
    for _ in 0..40 {
        export.extend([0x74, 0x00]);
    }
    export.extend([0x90; 100]);
    export.push(0xC3);
    assert_eq!(
        table(
            PointerSize::Bit64,
            "il2cpp_image_get_class",
            &[(0x1000, &export), (0x2000, &accessor_x64(0x2000))]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn queues_a_function_called_many_times_once() {
    // The export calls the same helper 600 times and then calls the accessor.
    let mut export = Vec::new();
    for i in 0..600 {
        export.extend([&[0xE8][..], &displacement(0x1000 + i * 5 + 5, 0x3C00)].concat());
    }
    export.extend([&[0xE8][..], &displacement(0x1000 + 600 * 5 + 5, 0x2400)].concat());
    export.push(0xC3);
    assert_eq!(
        table(
            PointerSize::Bit64,
            "il2cpp_image_get_class",
            &[
                (0x1000, &export),
                (0x2400, &accessor_x64(0x2400)),
                (0x3C00, &[0xC3])
            ]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn finds_the_table_behind_a_branch() {
    // The export skips over a return with a short branch, and the accessor's
    // load comes right after the return.
    let code = [&[0x74, 0x01, 0xC3][..], &accessor_x64(0x1003)].concat();
    assert_eq!(
        table(
            PointerSize::Bit64,
            "il2cpp_image_get_class",
            &[(0x1000, &code)]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn ends_on_a_loop() {
    // The function branches back to its start forever. It holds no table, but
    // the walk still ends.
    let code = [0x90, 0x75, 0xFD, 0xC3];
    assert_eq!(
        table(
            PointerSize::Bit64,
            "il2cpp_image_get_class",
            &[(0x1000, &code)]
        ),
        None
    );
}

/// Writes an exception directory, the table x64 images keep of where each
/// function starts and ends, at 0x300 of an image.
fn put_functions(image: &mut [u8], functions: &[(u32, u32)]) {
    const TABLE: usize = 0x300;
    image[0x120..0x124].copy_from_slice(&(TABLE as u32).to_le_bytes());
    image[0x124..0x128].copy_from_slice(&(functions.len() as u32 * 12).to_le_bytes());
    for (index, (begin, end)) in functions.iter().enumerate() {
        let at = TABLE + index * 12;
        image[at..at + 4].copy_from_slice(&begin.to_le_bytes());
        image[at + 4..at + 8].copy_from_slice(&end.to_le_bytes());
    }
}

#[test]
fn stops_at_the_end_of_a_function_after_a_call_that_does_not_return() {
    // The export calls the getter and ends there, since the call never
    // returns. The next function loads some other global the same way.
    let call = [&[0xE8][..], &displacement(0x1005, 0x1200)].concat();
    let next = [
        &[0x48, 0x8D, 0x05][..],
        &displacement(0x100C, 0x3800),
        &[0xC3],
    ]
    .concat();
    let getter = [
        &[0x48, 0x8D, 0x05][..],
        &displacement(0x1207, 0x3000),
        &[0xC3],
    ]
    .concat();
    let mut image = image(
        PointerSize::Bit64,
        &[("il2cpp_domain_get_assemblies", 0x1000)],
        &[(0x1000, &call), (0x1005, &next), (0x1200, &getter)],
    );
    put_functions(
        &mut image,
        &[(0x1000, 0x1005), (0x1005, 0x100D), (0x1200, 0x1208)],
    );
    let found = with_process(&[(BASE, &image)], |process| {
        globals::assemblies(
            process,
            (Address::new(BASE), SIZE as u64),
            PointerSize::Bit64,
        )
    });
    assert_eq!(found, Some(Address::new(BASE + 0x3000)));
}

#[test]
fn counts_a_jump_inside_a_function_as_a_branch() {
    // 4 tail jumps lead to the accessor, which starts with a short jump to
    // its own next instruction. Only a jump out of a function costs a level.
    let mut code: Vec<(u32, Vec<u8>)> = (0..4)
        .map(|hop| {
            (
                0x1000 + hop * 0x100,
                jmp(0x1000 + hop * 0x100, 0x1100 + hop * 0x100),
            )
        })
        .collect();
    code.push((0x1400, [&[0xEB, 0x00][..], &accessor_x64(0x1402)].concat()));
    let code: Vec<(u32, &[u8])> = code.iter().map(|(at, bytes)| (*at, &bytes[..])).collect();
    let mut image = image(
        PointerSize::Bit64,
        &[("il2cpp_image_get_class", 0x1000)],
        &code,
    );
    put_functions(
        &mut image,
        &[
            (0x1000, 0x1005),
            (0x1100, 0x1105),
            (0x1200, 0x1205),
            (0x1300, 0x1305),
            (0x1400, 0x1410),
        ],
    );
    let found = with_process(&[(BASE, &image)], |process| {
        globals::type_info_definition_table(
            process,
            (Address::new(BASE), SIZE as u64),
            PointerSize::Bit64,
        )
    });
    assert_eq!(found, Some(Address::new(BASE + 0x3000)));
}

#[test]
fn skips_a_load_of_something_outside_the_module() {
    // The export first calls a function that loads 1, then the getter.
    let calls = [
        &[0xE8][..],
        &displacement(0x1005, 0x1100),
        &[0xE8],
        &displacement(0x100A, 0x1200),
        &[0xC3],
    ]
    .concat();
    let one = [0xB8, 0x01, 0x00, 0x00, 0x00, 0xC3];
    let getter = [&[0xB8][..], &absolute(0x3000), &[0xC3]].concat();
    assert_eq!(
        assemblies(
            PointerSize::Bit32,
            &[(0x1000, &calls), (0x1100, &one), (0x1200, &getter)]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn needs_a_lea_after_a_load_to_count_it_as_the_table() {
    // A load followed by a jge whose displacement starts with the same bytes
    // as a lea's ModRM and SIB, then the real accessor.
    let decoy = [
        &[0x48, 0x8B, 0x05][..],
        &displacement(0x1007, 0x3800),
        &[0x0F, 0x8D, 0x34, 0x00, 0x00, 0x00],
    ]
    .concat();
    let code = [decoy, accessor_x64(0x100D)].concat();
    assert_eq!(
        table(
            PointerSize::Bit64,
            "il2cpp_image_get_class",
            &[(0x1000, &code)]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn x86_master_export_does_not_follow_a_call_in_the_next_function() {
    // mov eax, [s_Assemblies]; pop ebp; ret, then a function that calls
    // some other getter.
    let master = [&[0xA1][..], &absolute(0x3000), &[0x5D, 0xC3]].concat();
    let next = [&[0xE8][..], &displacement(0x1015, 0x1100), &[0xC3]].concat();
    let other_getter = [&[0xB8][..], &absolute(0x3800), &[0xC3]].concat();
    assert_eq!(
        assemblies(
            PointerSize::Bit32,
            &[(0x1000, &master), (0x1010, &next), (0x1100, &other_getter)]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn x86_type_export_does_not_match_an_indexed_load_in_the_next_function() {
    // The export jumps to the accessor, and the function after the jump
    // loads some other table the same way.
    let decoy = [
        &[0xA1][..],
        &absolute(0x3800),
        &[0x83, 0x3C, 0xB0, 0x00, 0xC3],
    ]
    .concat();
    let accessor = [&[0xA1][..], &absolute(0x3000), &[0x83, 0x3C, 0xB0, 0x00]].concat();
    assert_eq!(
        table(
            PointerSize::Bit32,
            "il2cpp_image_get_class",
            &[
                (0x1000, &jmp(0x1000, 0x1200)),
                (0x1005, &decoy),
                (0x1200, &accessor)
            ]
        ),
        Some(Address::new(BASE + 0x3000))
    );
}

#[test]
fn reads_code_before_the_code_it_already_read() {
    // The export sits after the accessor and jumps back to it, further than
    // one block of read code reaches.
    let image = image(
        PointerSize::Bit64,
        &[("il2cpp_image_get_class", 0x1800)],
        &[
            (0x1800, &jmp(0x1800, 0x1000)),
            (0x1000, &accessor_x64(0x1000)),
        ],
    );
    let found = with_process(&[(BASE, &image)], |process| {
        globals::type_info_definition_table(
            process,
            (Address::new(BASE), SIZE as u64),
            PointerSize::Bit64,
        )
    });
    assert_eq!(found, Some(Address::new(BASE + 0x3000)));
}

#[test]
fn follows_a_conditional_tail_jump_outside_the_function_range() {
    let branch = [&[0x0F, 0x85][..], &displacement(0x1006, 0x1200), &[0xC3]].concat();
    let mut image = image(
        PointerSize::Bit64,
        &[("il2cpp_image_get_class", 0x1000)],
        &[(0x1000, &branch), (0x1200, &accessor_x64(0x1200))],
    );
    put_functions(&mut image, &[(0x1000, 0x1007), (0x1200, 0x120D)]);
    let found = with_process(&[(BASE, &image)], |process| {
        globals::type_info_definition_table(
            process,
            (Address::new(BASE), SIZE as u64),
            PointerSize::Bit64,
        )
    });
    assert_eq!(found, Some(Address::new(BASE + 0x3000)));
}

#[test]
fn does_not_follow_a_call_byte_inside_a_mov_immediate() {
    let export = [
        &[0x48, 0xB8, 0xE8][..],
        &displacement(0x1007, 0x1100),
        &[0x00, 0x00, 0x00, 0xC3],
    ]
    .concat();
    let decoy = [
        &[0x48, 0x8D, 0x05][..],
        &displacement(0x1107, 0x3800),
        &[0xC3],
    ]
    .concat();
    assert_eq!(
        assemblies(PointerSize::Bit64, &[(0x1000, &export), (0x1100, &decoy)]),
        None
    );
}
