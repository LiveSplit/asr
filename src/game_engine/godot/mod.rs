//! Support for games using the Godot engine.
//!
//! The support is still very experimental. Currently only games using Godot 4.2
//! without any debug symbols are supported.
//!
//! The main entry point is [`SceneTree::locate`], which locates the
//! [`SceneTree`] instance in the game's memory. From there you can find the
//! root node and all its child nodes.
//!
//! # Example
//!
//! ```no_run
//! # async fn example(process: asr::Process, main_module_address: asr::Address) {
//! use asr::game_engine::godot::SceneTree;
//!
//! // We first locate the SceneTree instance.
//! let scene_tree = SceneTree::wait_locate(&process, main_module_address).await;
//!
//! // We access the root node of the SceneTree.
//! let root = scene_tree.wait_get_root(&process).await;
//!
//! // We print the tree of nodes starting from the root.
//! asr::print_limited::<4096>(&root.print_tree::<64>(&process));
//! # }

macro_rules! extends {
    ($Sub:ident: $Base:ident) => {
        impl core::ops::Deref for crate::game_engine::godot::Ptr<$Sub> {
            type Target = crate::game_engine::godot::Ptr<$Base>;

            fn deref(&self) -> &Self::Target {
                bytemuck::cast_ref(self)
            }
        }
    };
}

mod core;
mod scene;

pub use core::*;
pub use scene::*;

mod cpp;
pub use cpp::*;
