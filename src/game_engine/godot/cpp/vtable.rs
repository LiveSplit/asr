use crate::{Error, Process};

use super::{Ptr, TypeInfo};

/// A C++ virtual method table.
///
/// This can be used to look up virtual functions and type information for the
/// object. A pointer to a vtable is unique for each type, so comparing pointers
/// is enough to check for type equality.
///
/// [Wikipedia](https://en.wikipedia.org/wiki/Virtual_method_table)
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct VTable;

impl Ptr<VTable> {
    /// Queries information of a type. Used where the dynamic type of a
    /// polymorphic object must be known and for static type identification.
    ///
    /// [`typeid`](https://en.cppreference.com/w/cpp/language/typeid)
    pub fn get_type_info(self, process: &Process) -> Result<Ptr<TypeInfo>, Error> {
        self.read_at_offset(-8, process)
    }
}
