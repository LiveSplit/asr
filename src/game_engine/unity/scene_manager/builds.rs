//! Known Unity players and the layout of their scene manager, scenes,
//! transforms and game objects. Each entry is one measured player. The full
//! Unity version says which player. The offsets were read off the player's
//! code through the functions listed in its PDB, and the offsets and shapes
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
/// the four parts of the file version of `UnityPlayer.dll`, or of the game's
/// executable before Unity 2017.2, and the first three of them are the
/// patch. A player takes the newest build whose patch is at or below its own
/// patch. A player under every build takes the oldest build in the table.
pub(super) fn nearest(
    unity: (u16, u16, u16, u16),
    pointer_size: PointerSize,
) -> Option<&'static Build> {
    let at_size = || {
        BUILDS
            .iter()
            .filter(move |build| build.profile.pointer_size == pointer_size)
    };

    // The comparison drops the build number, the fourth part, because a later
    // build of one patch can carry a lower number than an earlier build.
    let patch = |version: (u16, u16, u16, u16)| (version.0, version.1, version.2);

    // The table reads from the oldest player to the newest, so the newest
    // build at or under the player's patch is the last one under it.
    at_size()
        .rfind(|build| patch(build.unity) <= patch(unity))
        .or_else(|| at_size().next())
}

// A function that loads the global into r13 or r14 right after its
// prologue. It hits once on every player from Unity 5.6 through 6000.1.
const PROLOGUE_LOAD_X64: Anchor = Anchor {
    signature: Signature::new(
        "48 83 EC 20 4C 8B ?5 ?? ?? ?? ?? 33 F6 ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ??",
    ),
    displacement: 7,
};

// The scene count getter of Unity 6: it loads the global into rax and reads
// the count at 0x18.
const SCENE_COUNT_GETTER_X64: Anchor = Anchor {
    signature: Signature::new(
        "48 8B 05 ?? ?? ?? ?? 8B 40 18 C3 ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ??",
    ),
    displacement: 3,
};

// A function that loads the global into ecx, pushes ebx, takes the address
// of the scene list and clears ebx. The offset of the scene list is the one
// byte that changes between Unity 5.6 and 2017.1, so it is left open. It
// hits once on every player from Unity 5.6 through 2017.2.
const LOAD_AND_CLEAR_ECX_X86: Anchor = Anchor {
    signature: Signature::new(
        "55 8B EC 83 EC 08 8B 0D ?? ?? ?? ?? 53 8D 41 ?? 33 DB 89 45 F8 39 18 74",
    ),
    displacement: 8,
};

// The same function once the global lands in eax and is stored in a local
// before the clear. It hits once on every player from Unity 2017.3 through
// 2022.1.
const LOAD_AND_CLEAR_X86: Anchor = Anchor {
    signature: Signature::new(
        "A1 ?? ?? ?? ?? 53 33 DB 89 45 FC ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ??",
    ),
    displacement: 1,
};

// The active scene getter from Unity 2022.2 on: it loads the global
// into eax and reads the active scene at 0x28.
const ACTIVE_SCENE_GETTER_X86: Anchor = Anchor {
    signature: Signature::new(
        "A1 ?? ?? ?? ?? 8B 48 28 ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ??",
    ),
    displacement: 1,
};

// The scene at index getter from Unity 2022.2 on: it loads the global into
// eax and compares the index with the scene count at 0x10.
const SCENE_AT_GETTER_X86: Anchor = Anchor {
    signature: Signature::new(
        "A1 ?? ?? ?? ?? 3B 50 10 ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ??",
    ),
    displacement: 1,
};

// The table reads from the oldest player to the newest.
pub(super) const BUILDS: &[Build] = &[
    // Unity 5.6.0f1, x64.
    Build {
        unity: (5, 6, 0, 23754),
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
    // Unity 5.6.0f1, x86.
    Build {
        unity: (5, 6, 0, 23754),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            anchor: LOAD_AND_CLEAR_ECX_X86,
            path: PathShape::Pointer,
            reference: ReferenceShape::CachedObject,
            manager: ManagerOffsets {
                scenes: 0x4,
                active_scene: 0x24,
                dont_destroy_on_load_scene: 0x38,
            },
            scene: SceneOffsets {
                path: 0x10,
                build_index: 0x74,
                roots: 0x8c,
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
    // Unity 2017.1.0f1, x64.
    Build {
        unity: (2017, 1, 0, 32737),
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
    // Unity 2017.1.0f1, x86.
    Build {
        unity: (2017, 1, 0, 32737),
        profile: Profile {
            pointer_size: PointerSize::Bit32,
            anchor: LOAD_AND_CLEAR_ECX_X86,
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
    // Unity 2017.3.0f1, x86.
    Build {
        unity: (2017, 3, 0, 20311),
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
    // Unity 2018.2.0f1, x64. UnityScene takes a base class with a vtable
    // pointer at its start here, so every member of a scene sits one pointer
    // further along than it does in 2018.1 and in 2018.3.
    Build {
        unity: (2018, 2, 0, 44229),
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
    // Unity 2018.2.0f1, x86.
    Build {
        unity: (2018, 2, 0, 44229),
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
                path: 0x10,
                build_index: 0x74,
                roots: 0x8c,
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
    // Unity 2018.3.0f1, x64.
    Build {
        unity: (2018, 3, 0, 9156),
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
    // Unity 2018.3.0f1, x86.
    Build {
        unity: (2018, 3, 0, 9156),
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
    // Unity 2021.1.0f1, x64.
    Build {
        unity: (2021, 1, 0, 42313),
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
    // Unity 2022.2.0f1, x86.
    Build {
        unity: (2022, 2, 0, 56532),
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
    // Unity 2022.3.5f1, x64.
    Build {
        unity: (2022, 3, 5, 29734),
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
    // Unity 2022.3.5f1, x86.
    Build {
        unity: (2022, 3, 5, 29734),
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
                roots: 0xac,
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
    // Unity 2023.1.0f1, x64.
    Build {
        unity: (2023, 1, 0, 2298),
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
    // Unity 2023.1.0f1, x86.
    Build {
        unity: (2023, 1, 0, 2298),
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
                roots: 0x70,
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
    // Unity 2023.1.4f1, x64.
    Build {
        unity: (2023, 1, 4, 6702),
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
    // Unity 2023.1.4f1, x86.
    Build {
        unity: (2023, 1, 4, 6702),
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
    // Unity 2023.2.0f1, x64.
    Build {
        unity: (2023, 2, 0, 54845),
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
    // Unity 2023.2.0f1, x86.
    Build {
        unity: (2023, 2, 0, 54845),
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
    // Unity 6000.0.59f2, x86.
    Build {
        unity: (6000, 0, 59, 10268),
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
    // Unity 6000.1.0f1, x86.
    Build {
        unity: (6000, 1, 0, 41298),
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
    // Unity 6000.2.0f1, x64. The layout is the one 2023.2 starts. The entry
    // is here because the prologue load stops hitting at 6000.2.0, so the
    // players from here on reach the manager through the scene count getter.
    Build {
        unity: (6000, 2, 0, 53701),
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
    // Unity 6000.2.2f1, x86.
    Build {
        unity: (6000, 2, 2, 14734),
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
    // Unity 6000.3.0f1, x64.
    Build {
        unity: (6000, 3, 0, 34572),
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
    // Unity 6000.3.0f1, x86.
    Build {
        unity: (6000, 3, 0, 34572),
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
    // Unity 6000.5.0f1, x64.
    Build {
        unity: (6000, 5, 0, 46204),
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
    // Unity 6000.5.0f1, x86.
    Build {
        unity: (6000, 5, 0, 46204),
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
        signature: Signature::new(
            "41 54 53 50 4C 8B ?5 ?? ?? ?? ?? 41 83 ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ??",
        ),
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
