use crate::{signature::Signature, PointerSize};

/// How the scene manager global is found: the body of a function that loads
/// the global, with the displacement of the load masked out, and where that
/// displacement starts inside a match.
pub(super) struct Anchor {
    pub(super) signature: Signature<13>,
    pub(super) displacement: u8,
}

/// How a scene keeps its path in the 32 bytes of its path field.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum PathShape {
    /// A pointer to the characters.
    Pointer,
    /// A path of up to 31 characters sits inline, with a NUL after it. A
    /// longer path sits behind a pointer, followed by its length and its
    /// capacity.
    InlineNul,
    /// Like [`InlineNul`](Self::InlineNul), but the last byte of the field
    /// holds 31 minus the length while the path is inline.
    InlineSpare,
}

/// The members of `RuntimeSceneManager` the walk reads. The loaded scenes
/// are a dynamic array, with the pointer to the scenes at `scenes` and the
/// count two pointers after it. The `DontDestroyOnLoad` scene is embedded in
/// the manager by value.
pub(super) struct ManagerOffsets {
    pub(super) scenes: u8,
    pub(super) active_scene: u8,
    pub(super) dont_destroy_on_load_scene: u8,
}

/// The members of `UnityScene` the walk reads. The roots are a circular list
/// whose head is embedded in the scene at `roots`.
pub(super) struct SceneOffsets {
    pub(super) path: u8,
    pub(super) build_index: u8,
    pub(super) roots: u16,
}

/// The members of `Transform` the walk reads. The game object is a member of
/// the `Component` base. The children are a dynamic array of transforms.
pub(super) struct TransformOffsets {
    pub(super) game_object: u8,
    pub(super) children: u8,
}

/// The members of `GameObject` the walk reads. The components are a dynamic
/// array of pairs, each a type index and a pointer to the component.
pub(super) struct GameObjectOffsets {
    pub(super) components: u8,
    pub(super) name: u8,
}

/// The members of `Object` the walk reads, which every component starts
/// with. The managed reference leads to the managed object of the component.
pub(super) struct ObjectOffsets {
    pub(super) managed_reference: u8,
}

/// Everything the walk needs to know about one player: its pointer size, how
/// the scene manager is found, how a scene keeps its path, and the offsets
/// of the engine structs.
pub(super) struct Profile {
    pub(super) pointer_size: PointerSize,
    pub(super) anchor: Anchor,
    pub(super) path: PathShape,
    pub(super) manager: ManagerOffsets,
    pub(super) scene: SceneOffsets,
    pub(super) transform: TransformOffsets,
    pub(super) game_object: GameObjectOffsets,
    pub(super) object: ObjectOffsets,
}
