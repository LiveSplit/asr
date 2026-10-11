use crate::{signature::Signature, PointerSize};

/// How the scene manager global is found: the body of a function that loads
/// the global, with the displacement of the load masked out, and where that
/// displacement starts inside a match. The signature always holds 24 bytes,
/// so a shorter body is written out with wildcards after it.
#[derive(Copy, Clone, Debug)]
pub struct Anchor {
    /// The body of the function, with the displacement masked out.
    pub signature: Signature<24>,
    /// Where the displacement of the load starts inside a match.
    pub displacement: u8,
}

/// How a scene keeps its path in the 32 bytes of its path field.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PathShape {
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

/// How a component reaches its managed object through the managed reference
/// of its `Object` base.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ReferenceShape {
    /// The reference holds the managed object.
    CachedObject,
    /// The reference points at a slot that holds the managed object.
    RootSlot,
}

/// The members of `RuntimeSceneManager` the walk reads. The loaded scenes
/// are a dynamic array, with the pointer to the scenes at `scenes` and the
/// count two pointers after it. The `DontDestroyOnLoad` scene is embedded in
/// the manager by value. Unity 5.6.0 and 5.6.1 have no such scene, so it is
/// `None` for them.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ManagerOffsets {
    /// The pointer to the loaded scenes.
    pub scenes: u8,
    /// The pointer to the active scene.
    pub active_scene: u8,
    /// The `DontDestroyOnLoad` scene, when the build has one.
    pub dont_destroy_on_load_scene: Option<u8>,
}

/// The members of `UnityScene` the walk reads. The roots are a circular list
/// whose head is embedded in the scene at `roots`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SceneOffsets {
    /// The 32-byte path field, read by the [`PathShape`] of the build.
    pub path: u8,
    /// The build index.
    pub build_index: u8,
    /// The head of the list of root transforms.
    pub roots: u16,
}

/// The members of `Transform` the walk reads. The game object is a member of
/// the `Component` base. The children are a dynamic array of transforms.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TransformOffsets {
    /// The pointer to the game object.
    pub game_object: u8,
    /// The pointer to the child transforms.
    pub children: u8,
}

/// The members of `GameObject` the walk reads. The components are a dynamic
/// array of pairs, each a type index and a pointer to the component.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GameObjectOffsets {
    /// The pointer to the components.
    pub components: u8,
    /// The pointer to the name.
    pub name: u8,
}

/// The members of `Object` the walk reads, which every component starts
/// with. The managed reference leads to the managed object of the component
/// by the [`ReferenceShape`] of the build.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ObjectOffsets {
    /// The managed reference.
    pub managed_reference: u8,
}

/// A complete measured layout of the scene manager and the objects it
/// reaches, with the anchor that finds the manager. Built-in measured
/// profiles are available in [`profiles`](super::profiles).
///
/// A custom profile lists the complete layout. Every field is written out on
/// purpose: struct update syntax would let the custom profile silently take
/// fields that a later release adds to the source profile.
///
/// ```
/// use asr::{
///     game_engine::unity::scene_manager::{
///         Anchor, GameObjectOffsets, ManagerOffsets, ObjectOffsets, PathShape, Profile,
///         ReferenceShape, SceneOffsets, TransformOffsets,
///     },
///     signature::Signature,
///     PointerSize,
/// };
///
/// const CUSTOM_PROFILE: Profile = Profile {
///     pointer_size: PointerSize::Bit64,
///     anchor: Anchor {
///         signature: Signature::new(
///             "48 8B 05 ?? ?? ?? ?? 8B 40 18 C3 ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ??",
///         ),
///         displacement: 3,
///     },
///     path: PathShape::InlineSpare,
///     reference: ReferenceShape::RootSlot,
///     manager: ManagerOffsets {
///         scenes: 0x8,
///         active_scene: 0x48,
///         dont_destroy_on_load_scene: Some(0x70),
///     },
///     scene: SceneOffsets {
///         path: 0x10,
///         build_index: 0x98,
///         roots: 0xe8,
///     },
///     transform: TransformOffsets {
///         game_object: 0x20,
///         children: 0x48,
///     },
///     game_object: GameObjectOffsets {
///         components: 0x20,
///         name: 0x50,
///     },
///     object: ObjectOffsets {
///         managed_reference: 0x18,
///     },
/// };
/// ```
#[derive(Copy, Clone, Debug)]
pub struct Profile {
    /// The pointer width this profile was measured for.
    pub pointer_size: PointerSize,
    /// The anchor that finds the scene manager.
    pub anchor: Anchor,
    /// How a scene keeps its path.
    pub path: PathShape,
    /// How a component reaches its managed object.
    pub reference: ReferenceShape,
    /// Offsets within `RuntimeSceneManager`.
    pub manager: ManagerOffsets,
    /// Offsets within `UnityScene`.
    pub scene: SceneOffsets,
    /// Offsets within `Transform`.
    pub transform: TransformOffsets,
    /// Offsets within `GameObject`.
    pub game_object: GameObjectOffsets,
    /// Offsets within `Object`.
    pub object: ObjectOffsets,
}
