//! <https://github.com/godotengine/godot/blob/07cf36d21c9056fb4055f020949fb90ebd795afb/scene/main/node.h>

use core::fmt;

use crate::{
    game_engine::godot::{HashMap, Object, Ptr, String, StringName},
    Error, Process,
};

use super::SceneTree;

/// Base class for all scene objects.
///
/// [`Node`](https://docs.godotengine.org/en/4.2/classes/class_node.html)
///
/// Check the [`Ptr<Node>`] documentation to see all the methods you can call
/// on it.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Node;
extends!(Node: Object);

impl Ptr<Node> {
    /// Returns this node's parent node, or [`None`] if the node doesn't have a parent.
    ///
    /// [`Node.get_parent`](https://docs.godotengine.org/en/4.2/classes/class_node.html#class-node-method-get-parent)
    pub fn get_parent(self, process: &Process) -> Result<Option<Ptr<Self>>, Error> {
        self.read_at_byte_offset(0x128, process).map(
            |ptr: Ptr<Self>| {
                if ptr.is_null() {
                    None
                } else {
                    Some(ptr)
                }
            },
        )
    }

    /// The owner of this node. The owner must be an ancestor of this node. When
    /// packing the owner node in a `PackedScene`, all the nodes it owns are
    /// also saved with it.
    ///
    /// [`Node.get_owner`](https://docs.godotengine.org/en/4.2/classes/class_node.html#class-node-property-owner)
    pub fn get_owner(self, process: &Process) -> Result<Option<Ptr<Self>>, Error> {
        self.read_at_byte_offset(0x130, process).map(
            |ptr: Ptr<Self>| {
                if ptr.is_null() {
                    None
                } else {
                    Some(ptr)
                }
            },
        )
    }

    /// Finds the first descendant of this node whose name matches the name
    /// provided, returning [`None`] if no match is found. The matching is done
    /// against node names, not their paths. As such, it is case-sensitive.
    /// Unlike the Godot API, not wildcards are supported.
    ///
    /// [`Node.find_child`](https://docs.godotengine.org/en/4.2/classes/class_node.html#class-node-method-find-child)
    pub fn find_child<const N: usize>(
        self,
        name: &[u8; N],
        process: &Process,
    ) -> Result<Option<Ptr<Node>>, Error> {
        self.get_children()
            .get(name, process)?
            .map(|node| node.deref(process))
            .transpose()
    }

    /// Fetches a child node by its index. Each child node has an index relative
    /// its siblings (see [`get_index`](Self::get_index)). The first child is at
    /// index 0. If no child exists at the given index, this method returns an
    /// error.
    ///
    /// [`Node.get_child`](https://docs.godotengine.org/en/4.2/classes/class_node.html#class-node-method-get-child)
    ///
    /// # Warning
    ///
    /// Prefer not using this function in loops, it has to iterate over a linked
    /// list and is not actually O(1). Iterate over the children directly
    /// instead in that case. Only use this function if you know the specific
    /// index of the child node.
    pub fn get_child(self, idx: usize, process: &Process) -> Result<Ptr<Node>, Error> {
        self.get_children()
            .iter(process)
            .nth(idx)
            .ok_or(Error {})?
            .1
            .deref(process)
    }

    /// Returns the number of children of this node.
    ///
    /// [`Node.get_child_count`](https://docs.godotengine.org/en/4.2/classes/class_node.html#class-node-method-get-child-count)
    pub fn get_child_count(self, process: &Process) -> Result<u32, Error> {
        self.get_children().size(process)
    }

    /// Returns all children of this node inside a [`HashMap`].
    ///
    /// [`Node.get_children`](https://docs.godotengine.org/en/4.2/classes/class_node.html#class-node-method-get-children)
    pub fn get_children(self) -> Ptr<HashMap<StringName, Ptr<Node>>> {
        Ptr::new(self.addr() + 0x138)
    }

    /// Returns this node's order among its siblings. The first node's index is
    /// `0`. See also [`get_child`](Self::get_child).
    ///
    /// [`Node.get_index`](https://docs.godotengine.org/en/4.2/classes/class_node.html#class-node-method-get-index)
    pub fn get_index(self, process: &Process) -> Result<i32, Error> {
        self.read_at_byte_offset(0x1C4, process)
    }

    /// The name of the node. This name must be unique among the siblings (other
    /// child nodes from the same parent). When set to an existing sibling's
    /// name, the node is automatically renamed.
    ///
    /// [`Node.get_name`](https://docs.godotengine.org/en/4.2/classes/class_node.html#class-node-property-name)
    pub fn get_name<const N: usize>(self, process: &Process) -> Result<String<N>, Error> {
        let string_name: StringName = self.read_at_byte_offset(0x1D0, process)?;
        string_name.read(process)
    }

    /// Prints the node and its children, recursively. The node does not have to
    /// be inside the tree.
    ///
    /// [`Node.print_tree`](https://docs.godotengine.org/en/4.2/classes/class_node.html#class-node-method-print-tree)
    #[must_use]
    pub fn print_tree<const N: usize>(self, process: &Process) -> PrintTree<'_, N> {
        PrintTree(self, process)
    }

    /// Returns [`true`] if this node is currently inside [`SceneTree`]. See
    /// also [`get_tree`](Self::get_tree).
    ///
    /// [`Node.is_inside_tree`](https://docs.godotengine.org/en/4.2/classes/class_node.html#class-node-method-is-inside-tree)
    pub fn is_inside_tree(self, process: &Process) -> Result<bool, Error> {
        self.get_tree(process).map(|tree| tree.is_some())
    }

    /// Returns the [`SceneTree`] that contains this node. If this node is not
    /// inside the tree, returns [`None`]. See also
    /// [`is_inside_tree`](Self::is_inside_tree).
    ///
    /// [`Node.get_tree`](https://docs.godotengine.org/en/4.2/classes/class_node.html#class-node-method-get-tree)
    pub fn get_tree(self, process: &Process) -> Result<Option<Ptr<SceneTree>>, Error> {
        self.read_at_byte_offset(0x1D8, process).map(
            |ptr: Ptr<SceneTree>| {
                if ptr.is_null() {
                    None
                } else {
                    Some(ptr)
                }
            },
        )
    }
}

/// A recursive tree printer.
pub struct PrintTree<'p, const N: usize>(Ptr<Node>, &'p Process);

impl<'p, const N: usize> fmt::Debug for PrintTree<'p, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_map = f.debug_map();
        for (name, node) in self.0.get_children().iter(self.1) {
            self.print_key(&mut debug_map, name);
            match node.deref(self.1) {
                Ok(node) => debug_map.value(&PrintTree::<N>(node, self.1)),
                Err(_) => debug_map.value(&"<failed reading node>"),
            };
        }
        debug_map.finish()
    }
}

impl<'p, const N: usize> fmt::Display for PrintTree<'p, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#?}", self)
    }
}

impl<'p, const N: usize> PrintTree<'p, N> {
    #[inline(never)]
    fn print_key(&self, debug_map: &mut fmt::DebugMap<'_, '_>, name: Ptr<StringName>) {
        debug_map.key(
            &name
                .deref(self.1)
                .ok()
                .and_then(|name| Some(name.read::<N>(self.1).ok()?.to_array_string::<N>()))
                .as_deref()
                .unwrap_or("<failed reading name>"),
        );
    }
}
