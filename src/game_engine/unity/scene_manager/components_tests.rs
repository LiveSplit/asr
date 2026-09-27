//! Tests pinning how a transform reaches the managed objects of its
//! components. The game object keeps its components in a dynamic array of
//! pairs, a type index and a pointer to the component, with the transform
//! first. A component reaches its managed object through the managed
//! reference of its `Object` base: the reference holds the object on the
//! older players, and points at a slot that holds the object from 2023.1 on.

use super::{builds, offsets::ReferenceShape, SceneManager, Transform};
use crate::{runtime::mock::with_process, string::ArrayCString, Address, PointerSize, Process};
use std::vec;
use std::vec::Vec;

const BASE: u64 = 0x7FF6_1000_0000;
const TRANSFORM: u64 = 0x100;
const GAME_OBJECT: u64 = 0x200;
const PAIRS: u64 = 0x300;
const MARKER: u64 = 0x400;
const MOVER: u64 = 0x480;
const MARKER_OBJECT: u64 = 0x500;
const MOVER_OBJECT: u64 = 0x580;
const SLOTS: u64 = 0x600;

fn put(image: &mut [u8], at: u64, bytes: &[u8]) {
    let at = at as usize;
    image[at..at + bytes.len()].copy_from_slice(bytes);
}

fn ptr(image: &mut [u8], at: u64, target: u64) {
    put(image, at, &target.to_le_bytes());
}

fn manager(unity: (u16, u16, u16, u16)) -> SceneManager {
    SceneManager {
        pointer_size: PointerSize::Bit64,
        address: Address::new(BASE),
        profile: &builds::nearest(unity, PointerSize::Bit64).unwrap().profile,
    }
}

// A transform whose game object holds three components: the transform
// itself, a Marker and a Mover. The two managed objects sit behind the
// components' managed references, by the shape of the build.
fn image(manager: &SceneManager) -> Vec<u8> {
    let profile = manager.profile;
    let mut i = vec![0; 0x1000];
    ptr(
        &mut i,
        TRANSFORM + profile.transform.game_object as u64,
        BASE + GAME_OBJECT,
    );
    ptr(
        &mut i,
        GAME_OBJECT + profile.game_object.components as u64,
        BASE + PAIRS,
    );
    ptr(
        &mut i,
        GAME_OBJECT + profile.game_object.components as u64 + 16,
        3,
    );
    ptr(&mut i, PAIRS + 8, BASE + TRANSFORM);
    ptr(&mut i, PAIRS + 24, BASE + MARKER);
    ptr(&mut i, PAIRS + 40, BASE + MOVER);
    let reference = profile.object.managed_reference as u64;
    match profile.reference {
        ReferenceShape::CachedObject => {
            ptr(&mut i, MARKER + reference, BASE + MARKER_OBJECT);
            ptr(&mut i, MOVER + reference, BASE + MOVER_OBJECT);
        }
        ReferenceShape::RootSlot => {
            ptr(&mut i, MARKER + reference, BASE + SLOTS);
            ptr(&mut i, MOVER + reference, BASE + SLOTS + 8);
            ptr(&mut i, SLOTS, BASE + MARKER_OBJECT);
            ptr(&mut i, SLOTS + 8, BASE + MOVER_OBJECT);
        }
    }
    i
}

fn components(manager: &SceneManager) -> Vec<Address> {
    with_process(&[(BASE, &image(manager))], |process: &Process| {
        let transform = Transform {
            address: Address::new(BASE + TRANSFORM),
        };
        transform.components(process, manager).unwrap().collect()
    })
}

#[test]
fn builds_point_at_a_slot_from_2023_1() {
    for build in builds::BUILDS {
        let expected = if (build.unity.0, build.unity.1) < (2023, 1) {
            ReferenceShape::CachedObject
        } else {
            ReferenceShape::RootSlot
        };
        assert_eq!(build.profile.reference, expected, "{:?}", build.unity);
    }
}

#[test]
fn components_reach_their_managed_objects_through_the_cached_object() {
    let manager = manager((2018, 4, 36, 54151));
    assert_eq!(manager.profile.reference, ReferenceShape::CachedObject);
    assert_eq!(
        components(&manager),
        [
            Address::new(BASE + MARKER_OBJECT),
            Address::new(BASE + MOVER_OBJECT)
        ]
    );
}

#[test]
fn components_reach_their_managed_objects_through_the_slot() {
    let manager = manager((2023, 1, 22, 16744));
    assert_eq!(manager.profile.reference, ReferenceShape::RootSlot);
    assert_eq!(
        components(&manager),
        [
            Address::new(BASE + MARKER_OBJECT),
            Address::new(BASE + MOVER_OBJECT)
        ]
    );
}

// The class name of each managed object comes from the runtime module. The
// search takes the first object whose class carries the name.
#[test]
fn find_component_takes_the_object_whose_class_carries_the_name() {
    let manager = manager((2023, 1, 22, 16744));
    with_process(&[(BASE, &image(&manager))], |process: &Process| {
        let transform = Transform {
            address: Address::new(BASE + TRANSFORM),
        };
        let class_name_of = |object: Address| -> Option<ArrayCString<128>> {
            let mut name = ArrayCString::<128>::new();
            let text: &[u8] = match object.value() - BASE {
                MARKER_OBJECT => b"Marker",
                MOVER_OBJECT => b"Mover",
                _ => return None,
            };
            bytemuck::bytes_of_mut(&mut name)[..text.len()].copy_from_slice(text);
            Some(name)
        };
        assert_eq!(
            transform
                .find_component(process, &manager, "Mover", class_name_of)
                .unwrap(),
            Address::new(BASE + MOVER_OBJECT)
        );
        assert!(transform
            .find_component(process, &manager, "Camera", class_name_of)
            .is_err());
    });
}
