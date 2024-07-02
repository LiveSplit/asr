use crate::{string::ArrayCString, Address64, Error, Process};

use super::Ptr;

/// The class `TypeInfo` holds implementation-specific information about a
/// type, including the name of the type and means to compare two types for
/// equality or collating order. This is the class returned by
/// [`Ptr<VTable>::get_type_info`].
///
/// [`std::type_info`](https://en.cppreference.com/w/cpp/types/type_info)
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct TypeInfo;

impl Ptr<TypeInfo> {
    /// Returns a GCC/Clang mangled null-terminated character string containing
    /// the name of the type. No guarantees are given; in particular, the
    /// returned string can be identical for several types.
    ///
    /// [`std::type_info::name`](https://en.cppreference.com/w/cpp/types/type_info/name)
    pub fn get_mangled_name<const N: usize>(
        self,
        process: &Process,
    ) -> Result<ArrayCString<N>, Error> {
        let name_ptr: Address64 = self.read_at_offset(0x8, process)?;
        process.read(name_ptr)
    }

    /// Checks if the mangled name of the type matches the given string.
    pub fn matches_mangled_name<const N: usize>(
        self,
        mangled_name: &[u8; N],
        process: &Process,
    ) -> Result<bool, Error> {
        Ok(self.get_mangled_name::<N>(process)?.matches(mangled_name))
    }
}
