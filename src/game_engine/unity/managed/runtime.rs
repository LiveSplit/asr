use super::{Assemblies, ClassRef, Classes, ImageRef};
use crate::{Address, PointerSize, Process};

/// What the runtimes genuinely disagree on. Matching exhaustively is the point:
/// a runtime added later is a compile error at every place the two differ,
/// rather than a silent fall through to whichever arm came first.
pub enum Runtime {
    Mono(MonoRuntime),
    Il2Cpp(Il2CppRuntime),
}

/// Mono keeps its assemblies in a glib list, its classes in each image's hash
/// table, and its statics behind the class's vtable.
pub struct MonoRuntime {
    pub assemblies: Address,
    pub class_cache: u16,
    pub hash_table_size: u16,
    pub hash_table_table: u16,
    pub next_class_cache: u16,
    pub field_count: u16,
    pub runtime_info: u16,
    pub vtable_size: u16,
    pub vtable: u16,
    /// The older runtime keeps the static data in the vtable's own data slot,
    /// where the newer one stores it past the vtable's method pointer array.
    pub statics_in_vtable_data: bool,
}

/// IL2CPP keeps its assemblies in a vector, its classes in a table its images
/// slice into, and its statics on the class itself.
pub struct Il2CppRuntime {
    pub assemblies: Address,
    pub type_info_definition_table: Address,
    pub type_count: u16,
    pub metadata_handle: u16,
    /// The older lineage keeps the handle inline in the image, where the newer
    /// one keeps a pointer to it.
    pub handle_is_inline: bool,
    pub field_count: u16,
    pub static_fields: u16,
}

impl Runtime {
    /// Walks the assemblies the target has loaded.
    pub fn assemblies<'a>(
        &self,
        process: &'a Process,
        pointer_size: PointerSize,
    ) -> Assemblies<'a> {
        match self {
            Self::Mono(mono) => Assemblies::mono(process, pointer_size, mono),
            Self::Il2Cpp(il2cpp) => Assemblies::il2cpp(process, pointer_size, il2cpp),
        }
    }

    /// Walks the classes an image holds.
    pub fn classes<'a>(
        &self,
        process: &'a Process,
        pointer_size: PointerSize,
        image: ImageRef,
    ) -> Classes<'a> {
        match self {
            Self::Mono(mono) => Classes::mono(process, pointer_size, mono, image),
            Self::Il2Cpp(il2cpp) => Classes::il2cpp(process, pointer_size, il2cpp, image),
        }
    }

    /// Reads how many fields a class declares.
    pub fn field_count(&self, process: &Process, class: ClassRef) -> u64 {
        match self {
            Self::Mono(mono) => process
                .read::<i32>(class.address + mono.field_count)
                .ok()
                .filter(|&count| count > 0)
                .unwrap_or_default() as u64,
            // A generic definition stores u16::MAX here; no real class
            // declares that many fields.
            Self::Il2Cpp(il2cpp) => process
                .read::<u16>(class.address + il2cpp.field_count)
                .ok()
                .filter(|&count| count != u16::MAX)
                .unwrap_or_default() as u64,
        }
    }

    /// Reads the address a class's static field offsets are measured from.
    pub fn static_table(
        &self,
        process: &Process,
        pointer_size: PointerSize,
        class: ClassRef,
    ) -> Option<Address> {
        let slot = match self {
            Self::Mono(mono) => {
                let runtime_info = process
                    .read_pointer(class.address + mono.runtime_info, pointer_size)
                    .ok()
                    .filter(|address| !address.is_null())?;

                let vtables = process
                    .read_pointer(runtime_info + pointer_size as u64, pointer_size)
                    .ok()
                    .filter(|address| !address.is_null())?;

                if mono.statics_in_vtable_data {
                    vtables + mono.vtable_size
                } else {
                    let vtable_size = process.read::<u32>(class.address + mono.vtable_size).ok()?;

                    vtables + mono.vtable + (pointer_size as u64).wrapping_mul(vtable_size as u64)
                }
            }
            Self::Il2Cpp(il2cpp) => class.address + il2cpp.static_fields,
        };

        process
            .read_pointer(slot, pointer_size)
            .ok()
            .filter(|address| !address.is_null())
    }

    /// Reads the class a live object belongs to, which is how a polymorphic
    /// field's runtime type is found.
    pub fn object_class(
        &self,
        process: &Process,
        pointer_size: PointerSize,
        object: Address,
    ) -> Option<ClassRef> {
        let address = process
            .read_pointer(object, pointer_size)
            .ok()
            .filter(|address| !address.is_null())?;

        // Mono reaches the class through the object's vtable, where IL2CPP
        // heads the object with it.
        let address = match self {
            Self::Mono(_) => process
                .read_pointer(address, pointer_size)
                .ok()
                .filter(|address| !address.is_null())?,
            Self::Il2Cpp(_) => address,
        };

        Some(ClassRef::new(address))
    }
}
