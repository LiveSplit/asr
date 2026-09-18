//! Known Unity players and the layout of their scene manager, scenes,
//! transforms and game objects. Each entry is one measured player. The full
//! Unity version says which player. The offsets were read off the player's
//! code through the functions its PDB names, and the offsets and the shapes
//! were checked against a running player with a known scene.

use super::offsets::{
    Anchor, GameObjectOffsets, ManagerOffsets, ObjectOffsets, PathShape, Profile, ReferenceShape,
    SceneOffsets, TransformOffsets,
};
use crate::{signature::Signature, PointerSize};

/// One measured Unity player and the layout of its scene manager.
pub(super) struct Build {
    pub(super) unity: (u16, u16, u16, u16),
    pub(super) profile: Profile,
}

/// Finds the build for a player at its pointer size. The Unity version is
/// the four parts of the file version of `UnityPlayer.dll`. A measured
/// player gets its own build. Any other player gets the newest build whose
/// major.minor is at or below the player's major.minor, or the oldest build
/// when no build is below. The table reads from the oldest player to the
/// newest, so the newest match is the last match.
pub(super) fn nearest(
    unity: (u16, u16, u16, u16),
    pointer_size: PointerSize,
) -> Option<&'static Build> {
    let at_size = || {
        BUILDS
            .iter()
            .filter(move |build| build.profile.pointer_size == pointer_size)
    };

    at_size()
        .find(|build| build.unity == unity)
        .or_else(|| at_size().rfind(|build| (build.unity.0, build.unity.1) <= (unity.0, unity.1)))
        .or_else(|| at_size().next())
}

// A function that loads the global into r13 or r14 right after its
// prologue. It hits once on every player from Unity 2017.4 through 2023.1.
const PROLOGUE_LOAD_X64: Anchor = Anchor {
    signature: Signature::new("48 83 EC 20 4C 8B ?5 ?? ?? ?? ?? 33 F6"),
    displacement: 7,
};

// The scene count getter of Unity 6: it loads the global into rax and reads
// the count at 0x18.
const SCENE_COUNT_GETTER_X64: Anchor = Anchor {
    signature: Signature::new("48 8B 05 ?? ?? ?? ?? 8B 40 18 C3 ?? ??"),
    displacement: 3,
};

// A function that loads the global into eax, then pushes ebx, clears it and
// stores eax in a local. It hits once on every player from Unity 2017.4
// through 2021.3.
const LOAD_AND_CLEAR_X86: Anchor = Anchor {
    signature: Signature::new("A1 ?? ?? ?? ?? 53 33 DB 89 45 FC ?? ??"),
    displacement: 1,
};

// The active scene getter of Unity 2022.3 and 2023.1: it loads the global
// into eax and reads the active scene at 0x28.
const ACTIVE_SCENE_GETTER_X86: Anchor = Anchor {
    signature: Signature::new("A1 ?? ?? ?? ?? 8B 48 28 ?? ?? ?? ?? ??"),
    displacement: 1,
};

// The scene at index getter of Unity 6: it loads the global into eax and
// compares the index with the scene count at 0x10.
const SCENE_AT_GETTER_X86: Anchor = Anchor {
    signature: Signature::new("A1 ?? ?? ?? ?? 3B 50 10 ?? ?? ?? ?? ??"),
    displacement: 1,
};

// The table reads from the oldest player to the newest.
pub(super) const BUILDS: &[Build] = &[
    // Unity 2017.4.40f1, x64.
    Build {
        unity: (2017, 4, 40, 5126),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: PROLOGUE_LOAD_X64,
            path: PathShape::Pointer,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: 0x70,
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
    // Unity 2017.4.40f1, x86.
    Build {
        unity: (2017, 4, 40, 5126),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            anchor: LOAD_AND_CLEAR_X86,
            path: PathShape::Pointer,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x28,
                dont_destroy_on_load_scene: 0x40,
            },
            scene: SceneOffsets {
                path: 0xc,
                build_index: 0x70,
                roots: 0x88,
            },
            transform: TransformOffsets {
                game_object: 0x1c,
                children: 0x50,
            },
            game_object: GameObjectOffsets {
                components: 0x1c,
                name: 0x48,
            },
            object: ObjectOffsets {
                managed_reference: 0x18,
            },
        },
    },
    // Unity 2018.4.36f1, x64.
    Build {
        unity: (2018, 4, 36, 54151),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: PROLOGUE_LOAD_X64,
            path: PathShape::Pointer,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: 0x70,
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
    // Unity 2018.4.36f1, x86.
    Build {
        unity: (2018, 4, 36, 54151),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            anchor: LOAD_AND_CLEAR_X86,
            path: PathShape::Pointer,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x28,
                dont_destroy_on_load_scene: 0x40,
            },
            scene: SceneOffsets {
                path: 0xc,
                build_index: 0x70,
                roots: 0x88,
            },
            transform: TransformOffsets {
                game_object: 0x1c,
                children: 0x50,
            },
            game_object: GameObjectOffsets {
                components: 0x1c,
                name: 0x3c,
            },
            object: ObjectOffsets {
                managed_reference: 0x18,
            },
        },
    },
    // Unity 2021.1.29f1, x64.
    Build {
        unity: (2021, 1, 29, 10531),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: PROLOGUE_LOAD_X64,
            path: PathShape::InlineNul,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: 0x70,
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
    // Unity 2022.3.0f1, x64.
    Build {
        unity: (2022, 3, 0, 4507),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: PROLOGUE_LOAD_X64,
            path: PathShape::InlineNul,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: 0x70,
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
    // Unity 2022.3.0f1, x86.
    Build {
        unity: (2022, 3, 0, 4507),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            anchor: ACTIVE_SCENE_GETTER_X86,
            path: PathShape::Pointer,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x28,
                dont_destroy_on_load_scene: 0x40,
            },
            scene: SceneOffsets {
                path: 0xc,
                build_index: 0x70,
                roots: 0x88,
            },
            transform: TransformOffsets {
                game_object: 0x1c,
                children: 0x50,
            },
            game_object: GameObjectOffsets {
                components: 0x1c,
                name: 0x3c,
            },
            object: ObjectOffsets {
                managed_reference: 0x18,
            },
        },
    },
    // Unity 2023.1.22f1, x64.
    Build {
        unity: (2023, 1, 22, 16744),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: PROLOGUE_LOAD_X64,
            path: PathShape::InlineSpare,
            reference: ReferenceShape::RootSlot,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: 0x70,
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
    // Unity 2023.1.22f1, x86.
    Build {
        unity: (2023, 1, 22, 16744),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            anchor: ACTIVE_SCENE_GETTER_X86,
            path: PathShape::Pointer,
            reference: ReferenceShape::RootSlot,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x28,
                dont_destroy_on_load_scene: 0x40,
            },
            scene: SceneOffsets {
                path: 0xc,
                build_index: 0x58,
                roots: 0x94,
            },
            transform: TransformOffsets {
                game_object: 0x1c,
                children: 0x50,
            },
            game_object: GameObjectOffsets {
                components: 0x1c,
                name: 0x3c,
            },
            object: ObjectOffsets {
                managed_reference: 0x10,
            },
        },
    },
    // Unity 6000.0.84f1, x64.
    Build {
        unity: (6000, 0, 84, 43887),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: SCENE_COUNT_GETTER_X64,
            path: PathShape::InlineSpare,
            reference: ReferenceShape::RootSlot,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: 0x70,
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
    // Unity 6000.0.84f1, x86.
    Build {
        unity: (6000, 0, 84, 43887),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            anchor: SCENE_AT_GETTER_X86,
            path: PathShape::Pointer,
            reference: ReferenceShape::RootSlot,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x28,
                dont_destroy_on_load_scene: 0x40,
            },
            scene: SceneOffsets {
                path: 0xc,
                build_index: 0x58,
                roots: 0x98,
            },
            transform: TransformOffsets {
                game_object: 0x14,
                children: 0x48,
            },
            game_object: GameObjectOffsets {
                components: 0x14,
                name: 0x34,
            },
            object: ObjectOffsets {
                managed_reference: 0x10,
            },
        },
    },
    // Unity 6000.1.17f1, x86.
    Build {
        unity: (6000, 1, 17, 47571),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            anchor: SCENE_AT_GETTER_X86,
            path: PathShape::Pointer,
            reference: ReferenceShape::RootSlot,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x28,
                dont_destroy_on_load_scene: 0x40,
            },
            scene: SceneOffsets {
                path: 0xc,
                build_index: 0x58,
                roots: 0x94,
            },
            transform: TransformOffsets {
                game_object: 0x14,
                children: 0x48,
            },
            game_object: GameObjectOffsets {
                components: 0x14,
                name: 0x34,
            },
            object: ObjectOffsets {
                managed_reference: 0x10,
            },
        },
    },
    // Unity 6000.2.12f1, x86.
    Build {
        unity: (6000, 2, 12, 40285),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            anchor: SCENE_AT_GETTER_X86,
            path: PathShape::Pointer,
            reference: ReferenceShape::RootSlot,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x28,
                dont_destroy_on_load_scene: 0x40,
            },
            scene: SceneOffsets {
                path: 0xc,
                build_index: 0x58,
                roots: 0x98,
            },
            transform: TransformOffsets {
                game_object: 0x14,
                children: 0x48,
            },
            game_object: GameObjectOffsets {
                components: 0x14,
                name: 0x34,
            },
            object: ObjectOffsets {
                managed_reference: 0x10,
            },
        },
    },
    // Unity 6000.3.21f1, x64.
    Build {
        unity: (6000, 3, 21, 9777),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: SCENE_COUNT_GETTER_X64,
            path: PathShape::InlineSpare,
            reference: ReferenceShape::RootSlot,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: 0x70,
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
    // Unity 6000.3.21f1, x86.
    Build {
        unity: (6000, 3, 21, 9777),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            anchor: SCENE_AT_GETTER_X86,
            path: PathShape::Pointer,
            reference: ReferenceShape::RootSlot,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x28,
                dont_destroy_on_load_scene: 0x40,
            },
            scene: SceneOffsets {
                path: 0xc,
                build_index: 0x58,
                roots: 0x98,
            },
            transform: TransformOffsets {
                game_object: 0x14,
                children: 0x28,
            },
            game_object: GameObjectOffsets {
                components: 0x14,
                name: 0x34,
            },
            object: ObjectOffsets {
                managed_reference: 0x10,
            },
        },
    },
    // Unity 6000.5.10f1, x64.
    Build {
        unity: (6000, 5, 10, 54518),
        profile: Profile {
            pointer_size: PointerSize::Bit64,
            anchor: SCENE_COUNT_GETTER_X64,
            path: PathShape::InlineSpare,
            reference: ReferenceShape::RootSlot,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x48,
                dont_destroy_on_load_scene: 0x70,
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
    // Unity 6000.5.10f1, x86.
    Build {
        unity: (6000, 5, 10, 54518),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            anchor: SCENE_AT_GETTER_X86,
            path: PathShape::Pointer,
            reference: ReferenceShape::RootSlot,
            manager: ManagerOffsets {
                scenes: 0x8,
                active_scene: 0x28,
                dont_destroy_on_load_scene: 0x40,
            },
            scene: SceneOffsets {
                path: 0x10,
                build_index: 0x5c,
                roots: 0xc0,
            },
            transform: TransformOffsets {
                game_object: 0x20,
                children: 0x40,
            },
            game_object: GameObjectOffsets {
                components: 0x20,
                name: 0x40,
            },
            object: ObjectOffsets {
                managed_reference: 0x18,
            },
        },
    },
];

/// The layout used for a Linux or Mac x64 player. Those players are not
/// measured yet, so this keeps the signature and the offsets the walk used
/// before the table existed. The offsets are the x64 layout of Unity 2018.4
/// through 2022.3.
pub(super) const ELF_AND_MACHO_X64: Profile = Profile {
    pointer_size: PointerSize::Bit64,
    anchor: Anchor {
        signature: Signature::new("41 54 53 50 4C 8B ?5 ?? ?? ?? ?? 41 83"),
        displacement: 7,
    },
    path: PathShape::Pointer,
    reference: ReferenceShape::CachedObject,
    manager: ManagerOffsets {
        scenes: 0x8,
        active_scene: 0x48,
        dont_destroy_on_load_scene: 0x70,
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
};
