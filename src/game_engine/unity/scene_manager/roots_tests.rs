//! Tests pinning the walk over a scene's root transforms. The roots are a
//! circular list. Its head is embedded in the scene, and every element keeps
//! `prev` first and `next` second. A node keeps its transform after those.

use super::{builds, Scene, SceneManager};
use crate::{runtime::mock::with_process, Address, PointerSize, Process};
use std::vec;
use std::vec::Vec;

const BASE: u64 = 0x7FF6_1000_0000;
const SCENE: u64 = 0x100;

fn put(image: &mut [u8], at: u64, bytes: &[u8]) {
    let at = at as usize;
    image[at..at + bytes.len()].copy_from_slice(bytes);
}

fn ptr(image: &mut [u8], at: u64, target: u64) {
    put(image, at, &target.to_le_bytes());
}

fn manager() -> SceneManager {
    SceneManager {
        pointer_size: PointerSize::Bit64,
        address: Address::new(BASE),
        profile: &builds::nearest((6000, 3, 21, 9777), PointerSize::Bit64)
            .unwrap()
            .profile,
    }
}

// Lays a ring of nodes behind the head at `head`, each node holding one of
// the transforms, and links the head to the first and last nodes.
fn ring(image: &mut [u8], head: u64, nodes: &[(u64, u64)]) {
    let around = |index: usize| -> u64 {
        match nodes.get(index) {
            Some(&(node, _)) => BASE + node,
            None => BASE + head,
        }
    };
    ptr(image, head, around(nodes.len().wrapping_sub(1)));
    ptr(image, head + 8, around(0));
    for (index, &(node, transform)) in nodes.iter().enumerate() {
        ptr(image, node, around(index.wrapping_sub(1)));
        ptr(image, node + 8, around(index + 1));
        ptr(image, node + 16, BASE + transform);
    }
}

fn roots(image: &[u8]) -> Vec<Address> {
    let manager = manager();
    with_process(&[(BASE, image)], |process: &Process| {
        let scene = Scene {
            address: Address::new(BASE + SCENE),
        };
        manager
            .root_game_objects(process, &scene)
            .map(|transform| transform.address)
            .collect()
    })
}

#[test]
fn roots_come_out_first_to_last() {
    let mut image = vec![0; 0x1000];
    let head = SCENE + manager().profile.scene.roots as u64;
    ring(
        &mut image,
        head,
        &[(0x400, 0x800), (0x440, 0x840), (0x480, 0x880)],
    );

    assert_eq!(
        roots(&image),
        [
            Address::new(BASE + 0x800),
            Address::new(BASE + 0x840),
            Address::new(BASE + 0x880)
        ]
    );
}

#[test]
fn an_empty_ring_has_no_roots() {
    let mut image = vec![0; 0x1000];
    let head = SCENE + manager().profile.scene.roots as u64;
    ring(&mut image, head, &[]);

    assert_eq!(roots(&image), []);
}

#[test]
fn a_ring_that_does_not_close_stops_at_the_break() {
    let mut image = vec![0; 0x1000];
    let head = SCENE + manager().profile.scene.roots as u64;
    ring(&mut image, head, &[(0x400, 0x800), (0x440, 0x840)]);
    ptr(&mut image, 0x440 + 8, 0);

    assert_eq!(
        roots(&image),
        [Address::new(BASE + 0x800), Address::new(BASE + 0x840)]
    );
}
