//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/scene/main/scene_tree.h>

use crate::{
    future::retry,
    game_engine::godot::{MainLoop, Ptr},
    Address, Address64, Error, Process,
};

use super::{Node, Window};

#[allow(unused)]
mod offsets {
    use crate::{
        game_engine::godot::{HashMap, HashSet, List, SizeInTargetProcess},
        Address64,
    };

    // *const Window
    pub const ROOT: u64 = 0x2B0;
    // i64
    pub const CURRENT_FRAME: u64 = 0x330;
    // i32
    pub const NODES_IN_TREE_COUNT: u64 = 0x338;
    // bool
    pub const PROCESSING: u64 = 0x33C;
    // i32
    pub const NODES_REMOVED_ON_GROUP_CALL_LOCK: u64 = 0x340;
    // HashSet<*const Node>
    pub const NODES_REMOVED_ON_GROUP_CALL: u64 = 0x348;
    // List<ObjectId>
    pub const DELETE_QUEUE: u64 =
        (NODES_REMOVED_ON_GROUP_CALL + HashSet::<()>::SIZE).next_multiple_of(8);
    /// HashMap<UGCall, Vector<Variant>, UGCall>
    pub const UNIQUE_GROUP_CALLS: u64 = (DELETE_QUEUE + List::<()>::SIZE).next_multiple_of(8);
    // bool
    pub const UGC_LOCKED: u64 = UNIQUE_GROUP_CALLS + HashMap::<(), ()>::SIZE;
    // *const Node
    pub const CURRENT_SCENE: u64 = (UGC_LOCKED + 1).next_multiple_of(8);
    // *const Node
    pub const PREV_SCENE: u64 = CURRENT_SCENE + 8;
    // *const Node
    pub const PENDING_NEW_SCENE: u64 = PREV_SCENE + 8;
}

/// Manages the game loop via a hierarchy of nodes.
///
/// [`SceneTree`](https://docs.godotengine.org/en/4.2/classes/class_scenetree.html)
///
/// Check the [`Ptr<SceneTree>`] documentation to see all the methods you can call
/// on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct SceneTree;
extends!(SceneTree: MainLoop);

impl SceneTree {
    /// Locates the `SceneTree` instance in the given process.
    pub fn locate(process: &Process, main_module: Address) -> Result<Ptr<Self>, Error> {
        let addr: Address64 = process.read(main_module + 0x0424BE40)?;
        if addr.is_null() {
            return Err(Error {});
        }
        Ok(Ptr::new(addr))
    }

    /// Waits for the `SceneTree` instance to be located in the given process.
    pub async fn wait_locate(process: &Process, main_module: Address) -> Ptr<Self> {
        retry(|| Self::locate(process, main_module)).await
    }
}

impl Ptr<SceneTree> {
    /// The `SceneTree`'s root [`Window`].
    ///
    /// [`SceneTree.get_root`](https://docs.godotengine.org/en/4.2/classes/class_scenetree.html#class-scenetree-property-root)
    pub fn get_root(self, process: &Process) -> Result<Ptr<Window>, Error> {
        self.read_at_byte_offset(offsets::ROOT, process)
    }

    /// Waits for the `SceneTree`'s root [`Window`] to be available.
    pub async fn wait_get_root(self, process: &Process) -> Ptr<Window> {
        retry(|| self.get_root(process)).await
    }

    /// Returns the current frame number, i.e. the total frame count since the
    /// application started.
    ///
    /// [`SceneTree.get_frame`](https://docs.godotengine.org/en/4.2/classes/class_scenetree.html#class-scenetree-method-get-frame)
    pub fn get_frame(self, process: &Process) -> Result<i64, Error> {
        self.read_at_byte_offset(offsets::CURRENT_FRAME, process)
    }

    /// Returns the root node of the currently running scene, regardless of its
    /// structure.
    ///
    /// [`SceneTree.get_current_scene`](https://docs.godotengine.org/en/4.2/classes/class_scenetree.html#class-scenetree-property-current-scene)
    pub fn get_current_scene(self, process: &Process) -> Result<Option<Ptr<Node>>, Error> {
        let scene: Ptr<Node> = self.read_at_byte_offset(offsets::CURRENT_SCENE, process)?;
        Ok(if scene.is_null() { None } else { Some(scene) })
    }
}
