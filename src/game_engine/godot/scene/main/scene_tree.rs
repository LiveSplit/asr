//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/scene/main/scene_tree.h>

use crate::{
    future::retry,
    game_engine::godot::{MainLoop, Ptr},
    Address, Address64, Error, Process,
};

use super::Window;

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
    pub fn locate(process: &Process, module: Address) -> Result<Ptr<Self>, Error> {
        let addr: Address64 = process.read(module + 0x0424BE40)?;
        if addr.is_null() {
            return Err(Error {});
        }
        Ok(Ptr::new(addr))
    }

    /// Waits for the `SceneTree` instance to be located in the given process.
    pub async fn wait_locate(process: &Process, module: Address) -> Ptr<Self> {
        retry(|| Self::locate(process, module)).await
    }
}

impl Ptr<SceneTree> {
    /// The `SceneTree`'s root [`Window`].
    ///
    /// [`SceneTree.get_root`](https://docs.godotengine.org/en/4.2/classes/class_scenetree.html#class-scenetree-property-root)
    pub fn get_root(self, process: &Process) -> Result<Ptr<Window>, Error> {
        self.read_at_offset(0x2B0, process)
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
        self.read_at_offset(0x330, process)
    }
}
