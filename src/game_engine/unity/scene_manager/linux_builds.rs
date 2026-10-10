//! Known Linux Unity players and the layout of their scene manager, scenes,
//! transforms and game objects. Each entry is one measured x64 player. The
//! offsets were measured on a running player with a known scene. They equal
//! the offsets of the Windows x64 build that the same version takes, so only
//! the anchors differ.

use super::builds::{nearest_in, Build};
use super::offsets::{
    Anchor, GameObjectOffsets, ManagerOffsets, ObjectOffsets, PathShape, Profile, ReferenceShape,
    SceneOffsets, TransformOffsets,
};
use crate::{signature::Signature, PointerSize};

/// Finds the build for a Linux x64 player by the rule of [`nearest_in`]. The
/// Unity version comes from the version string in the engine module, so its
/// fourth part is zero.
pub(super) fn nearest(unity: (u16, u16, u16, u16)) -> Option<&'static Build> {
    nearest_in(BUILDS, unity, PointerSize::Bit64)
}

// A function that loads the global into rbp and checks the scene count at
// 0x18. It hits once on every player from Unity 5.6 through 2017.2.
const COUNT_CHECK_RBP_X64: Anchor = Anchor {
    signature: Signature::new(
        "41 54 49 89 FC 55 48 8B 2D ?? ?? ?? ?? 53 8B 45 18 85 C0 74 ?? ?? ?? ??",
    ),
    displacement: 9,
};

// The same function once it copies rdi after the load instead of before.
// It hits once on every player from Unity 2017.3 through 2018.1.
const COUNT_CHECK_RBP_SAVED_X64: Anchor = Anchor {
    signature: Signature::new(
        "41 54 55 48 8B 2D ?? ?? ?? ?? 53 8B 45 18 85 C0 74 ?? 49 89 FC 31 DB ??",
    ),
    displacement: 6,
};

// The same function once the global lands in r14. It hits once on every
// player from Unity 2018.2 through 2018.3.
const COUNT_CHECK_R14_X64: Anchor = Anchor {
    signature: Signature::new(
        "41 56 4C 8B 35 ?? ?? ?? ?? 41 55 49 89 FD 41 54 45 31 E4 55 53 41 8B 46",
    ),
    displacement: 5,
};

// The end of the function that creates the manager: it calls the
// constructor and stores the new manager in the global. It hits once on every
// player from Unity 2019.1 through 2019.2.
const MANAGER_STORE_X64: Anchor = Anchor {
    signature: Signature::new(
        "74 ?? BE 61 00 00 00 48 89 C7 E8 ?? ?? ?? ?? 48 89 1D ?? ?? ?? ?? 5B C3",
    ),
    displacement: 18,
};

// The count check function once it pushes rbp, r15, r14, r13, r12, rbx and
// rax, copies rdi into r15 and then loads the global into r14. It hits once
// on every player from Unity 2019.3 through 2020.1.
const COUNT_CHECK_SAVED_RDI_X64: Anchor = Anchor {
    signature: Signature::new(
        "55 41 57 41 56 41 55 41 54 53 50 49 89 FF 4C 8B 35 ?? ?? ?? ?? 41 83 7E",
    ),
    displacement: 17,
};

// The same function once it copies rdi after the load. It hits once on every
// player from Unity 2020.2 through 6000.5.
const COUNT_CHECK_PROLOGUE_X64: Anchor = Anchor {
    signature: Signature::new(
        "55 41 57 41 56 41 55 41 54 53 50 4C 8B 35 ?? ?? ?? ?? 41 83 7E 18 00 74",
    ),
    displacement: 14,
};

// A function that loads the global into rdi and calls through its vtable.
// It hits once on every player from Unity 6000.0 through 6000.6.
const VIRTUAL_CALL_X64: Anchor = Anchor {
    signature: Signature::new(
        "48 8B 3D ?? ?? ?? ?? 48 8B 07 FF 50 10 48 89 DF 48 89 C6 E8 ?? ?? ?? ??",
    ),
    displacement: 3,
};

// The table reads from the oldest player to the newest.
pub(super) const BUILDS: &[Build] = &[
    // Unity 5.6.0f1, x64.
    Build {
        unity: (5, 6, 0, 23754),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: COUNT_CHECK_RBP_X64,
            path: PathShape::Pointer,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: None,
            },
            scene: SceneOffsets {
                path: 0x18,
                build_index: 0xa0,
                roots: 0xb8,
            },
            transform: TransformOffsets {
                game_object: 0x30,
                children: 0x70,
            },
            game_object: GameObjectOffsets {
                components: 0x30,
                name: 0x60,
            },
            object: ObjectOffsets {
                managed_reference: 0x28,
            },
        },
    },
    // Unity 5.6.2f1, x64. Same layout as 5.6.0, but with the DontDestroyOnLoad
    // scene.
    Build {
        unity: (5, 6, 2, 37180),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: COUNT_CHECK_RBP_X64,
            path: PathShape::Pointer,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: Some(0x70),
            },
            scene: SceneOffsets {
                path: 0x18,
                build_index: 0xa0,
                roots: 0xb8,
            },
            transform: TransformOffsets {
                game_object: 0x30,
                children: 0x70,
            },
            game_object: GameObjectOffsets {
                components: 0x30,
                name: 0x60,
            },
            object: ObjectOffsets {
                managed_reference: 0x28,
            },
        },
    },
    // Unity 2017.1.0f1, x64.
    Build {
        unity: (2017, 1, 0, 32737),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: COUNT_CHECK_RBP_X64,
            path: PathShape::Pointer,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: Some(0x70),
            },
            scene: SceneOffsets {
                path: 0x10,
                build_index: 0x98,
                roots: 0xb0,
            },
            transform: TransformOffsets {
                game_object: 0x30,
                children: 0x70,
            },
            game_object: GameObjectOffsets {
                components: 0x30,
                name: 0x68,
            },
            object: ObjectOffsets {
                managed_reference: 0x28,
            },
        },
    },
    // Unity 2017.3.0f1, x64. Same layout as 2017.1.0, but the anchor changed.
    Build {
        unity: (2017, 3, 0, 20311),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: COUNT_CHECK_RBP_SAVED_X64,
            path: PathShape::Pointer,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: Some(0x70),
            },
            scene: SceneOffsets {
                path: 0x10,
                build_index: 0x98,
                roots: 0xb0,
            },
            transform: TransformOffsets {
                game_object: 0x30,
                children: 0x70,
            },
            game_object: GameObjectOffsets {
                components: 0x30,
                name: 0x68,
            },
            object: ObjectOffsets {
                managed_reference: 0x28,
            },
        },
    },
    // Unity 2018.2.0f1, x64.
    Build {
        unity: (2018, 2, 0, 44229),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: COUNT_CHECK_R14_X64,
            path: PathShape::Pointer,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: Some(0x70),
            },
            scene: SceneOffsets {
                path: 0x18,
                build_index: 0xa0,
                roots: 0xb8,
            },
            transform: TransformOffsets {
                game_object: 0x30,
                children: 0x70,
            },
            game_object: GameObjectOffsets {
                components: 0x30,
                name: 0x68,
            },
            object: ObjectOffsets {
                managed_reference: 0x28,
            },
        },
    },
    // Unity 2018.3.0f1, x64.
    Build {
        unity: (2018, 3, 0, 9156),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: COUNT_CHECK_R14_X64,
            path: PathShape::Pointer,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: Some(0x70),
            },
            scene: SceneOffsets {
                path: 0x10,
                build_index: 0x98,
                roots: 0xb0,
            },
            transform: TransformOffsets {
                game_object: 0x30,
                children: 0x70,
            },
            game_object: GameObjectOffsets {
                components: 0x30,
                name: 0x60,
            },
            object: ObjectOffsets {
                managed_reference: 0x28,
            },
        },
    },
    // Unity 2019.1.0f1, x64. Same layout as 2018.3.0, but the anchor changed.
    Build {
        unity: (2019, 1, 0, 21026),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: MANAGER_STORE_X64,
            path: PathShape::Pointer,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: Some(0x70),
            },
            scene: SceneOffsets {
                path: 0x10,
                build_index: 0x98,
                roots: 0xb0,
            },
            transform: TransformOffsets {
                game_object: 0x30,
                children: 0x70,
            },
            game_object: GameObjectOffsets {
                components: 0x30,
                name: 0x60,
            },
            object: ObjectOffsets {
                managed_reference: 0x28,
            },
        },
    },
    // Unity 2019.3.0f1, x64. Same layout as 2018.3.0, but the anchor changed.
    Build {
        unity: (2019, 3, 0, 44266),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: COUNT_CHECK_SAVED_RDI_X64,
            path: PathShape::Pointer,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: Some(0x70),
            },
            scene: SceneOffsets {
                path: 0x10,
                build_index: 0x98,
                roots: 0xb0,
            },
            transform: TransformOffsets {
                game_object: 0x30,
                children: 0x70,
            },
            game_object: GameObjectOffsets {
                components: 0x30,
                name: 0x60,
            },
            object: ObjectOffsets {
                managed_reference: 0x28,
            },
        },
    },
    // Unity 2020.2.0f1, x64. Same layout as 2018.3.0, but the anchor changed.
    Build {
        unity: (2020, 2, 0, 8671),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: COUNT_CHECK_PROLOGUE_X64,
            path: PathShape::Pointer,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: Some(0x70),
            },
            scene: SceneOffsets {
                path: 0x10,
                build_index: 0x98,
                roots: 0xb0,
            },
            transform: TransformOffsets {
                game_object: 0x30,
                children: 0x70,
            },
            game_object: GameObjectOffsets {
                components: 0x30,
                name: 0x60,
            },
            object: ObjectOffsets {
                managed_reference: 0x28,
            },
        },
    },
    // Unity 2021.1.0f1, x64.
    Build {
        unity: (2021, 1, 0, 42313),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: COUNT_CHECK_PROLOGUE_X64,
            path: PathShape::InlineNul,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: Some(0x70),
            },
            scene: SceneOffsets {
                path: 0x10,
                build_index: 0x98,
                roots: 0xb0,
            },
            transform: TransformOffsets {
                game_object: 0x30,
                children: 0x70,
            },
            game_object: GameObjectOffsets {
                components: 0x30,
                name: 0x60,
            },
            object: ObjectOffsets {
                managed_reference: 0x28,
            },
        },
    },
    // Unity 2022.3.5f1, x64.
    Build {
        unity: (2022, 3, 5, 29734),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: COUNT_CHECK_PROLOGUE_X64,
            path: PathShape::InlineNul,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: Some(0x70),
            },
            scene: SceneOffsets {
                path: 0x10,
                build_index: 0x98,
                roots: 0xe8,
            },
            transform: TransformOffsets {
                game_object: 0x30,
                children: 0x70,
            },
            game_object: GameObjectOffsets {
                components: 0x30,
                name: 0x60,
            },
            object: ObjectOffsets {
                managed_reference: 0x28,
            },
        },
    },
    // Unity 2023.1.0f1, x64.
    Build {
        unity: (2023, 1, 0, 2298),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: COUNT_CHECK_PROLOGUE_X64,
            path: PathShape::InlineSpare,
            reference: ReferenceShape::RootSlot,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: Some(0x70),
            },
            scene: SceneOffsets {
                path: 0x10,
                build_index: 0x98,
                roots: 0xb0,
            },
            transform: TransformOffsets {
                game_object: 0x30,
                children: 0x70,
            },
            game_object: GameObjectOffsets {
                components: 0x30,
                name: 0x60,
            },
            object: ObjectOffsets {
                managed_reference: 0x18,
            },
        },
    },
    // Unity 2023.1.4f1, x64.
    Build {
        unity: (2023, 1, 4, 6702),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: COUNT_CHECK_PROLOGUE_X64,
            path: PathShape::InlineSpare,
            reference: ReferenceShape::RootSlot,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: Some(0x70),
            },
            scene: SceneOffsets {
                path: 0x10,
                build_index: 0x98,
                roots: 0xe8,
            },
            transform: TransformOffsets {
                game_object: 0x30,
                children: 0x70,
            },
            game_object: GameObjectOffsets {
                components: 0x30,
                name: 0x60,
            },
            object: ObjectOffsets {
                managed_reference: 0x18,
            },
        },
    },
    // Unity 2023.2.0f1, x64.
    Build {
        unity: (2023, 2, 0, 54845),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: COUNT_CHECK_PROLOGUE_X64,
            path: PathShape::InlineSpare,
            reference: ReferenceShape::RootSlot,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: Some(0x70),
            },
            scene: SceneOffsets {
                path: 0x10,
                build_index: 0x98,
                roots: 0xe8,
            },
            transform: TransformOffsets {
                game_object: 0x20,
                children: 0x60,
            },
            game_object: GameObjectOffsets {
                components: 0x20,
                name: 0x50,
            },
            object: ObjectOffsets {
                managed_reference: 0x18,
            },
        },
    },
    // Unity 6000.3.0f1, x64.
    Build {
        unity: (6000, 3, 0, 34572),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: COUNT_CHECK_PROLOGUE_X64,
            path: PathShape::InlineSpare,
            reference: ReferenceShape::RootSlot,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: Some(0x70),
            },
            scene: SceneOffsets {
                path: 0x10,
                build_index: 0x98,
                roots: 0xe8,
            },
            transform: TransformOffsets {
                game_object: 0x20,
                children: 0x48,
            },
            game_object: GameObjectOffsets {
                components: 0x20,
                name: 0x50,
            },
            object: ObjectOffsets {
                managed_reference: 0x18,
            },
        },
    },
    // Unity 6000.5.0f1, x64.
    Build {
        unity: (6000, 5, 0, 46204),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: COUNT_CHECK_PROLOGUE_X64,
            path: PathShape::InlineSpare,
            reference: ReferenceShape::RootSlot,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: Some(0x70),
            },
            scene: SceneOffsets {
                path: 0x10,
                build_index: 0x98,
                roots: 0x110,
            },
            transform: TransformOffsets {
                game_object: 0x28,
                children: 0x50,
            },
            game_object: GameObjectOffsets {
                components: 0x28,
                name: 0x58,
            },
            object: ObjectOffsets {
                managed_reference: 0x20,
            },
        },
    },
    // Unity 6000.6.0f1, x64. Same layout as 6000.5.0, but the anchor changed.
    Build {
        unity: (6000, 6, 0, 63725),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: VIRTUAL_CALL_X64,
            path: PathShape::InlineSpare,
            reference: ReferenceShape::RootSlot,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: Some(0x70),
            },
            scene: SceneOffsets {
                path: 0x10,
                build_index: 0x98,
                roots: 0x110,
            },
            transform: TransformOffsets {
                game_object: 0x28,
                children: 0x50,
            },
            game_object: GameObjectOffsets {
                components: 0x28,
                name: 0x58,
            },
            object: ObjectOffsets {
                managed_reference: 0x20,
            },
        },
    },
];
