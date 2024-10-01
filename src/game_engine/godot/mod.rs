//! Support for games using the Godot engine.
//!
//! The support is still very experimental. Currently only games using Godot 4.2
//! without any debug symbols are supported.
//!
//! The main entry point is [`SceneTree::locate`], which locates the
//! [`SceneTree`] instance in the game's memory. From there you can find the
//! root node and all its child nodes. Nodes may also have attached scripts,
//! which can also be accessed and queried for their members.
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
//! ```
//!
//! # Extensibility
//!
//! The types and the code are closely matching the Godot source code. If there
//! is anything missing, chances are that it can easily be added. Feel free to
//! open an issue or contribute the missing parts yourself.
//!
//! # Copyright Notice
//!
//! Copyright (c) 2014-present Godot Engine contributors (see AUTHORS.md).
//!
//! Copyright (c) 2007-2014 Juan Linietsky, Ariel Manzur.
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to
//! deal in the Software without restriction, including without limitation the
//! rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
//! sell copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in
//! all copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
//! FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
//! IN THE SOFTWARE.

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
mod modules;
mod scene;

pub use core::*;
pub use modules::*;
pub use scene::*;

mod cpp;
pub use cpp::*;
