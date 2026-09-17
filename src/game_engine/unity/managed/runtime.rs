use super::{Assemblies, ClassRef, Classes, ImageRef};
use crate::{Address, PointerSize, Process};

/// What the runtimes genuinely disagree on. Matching exhaustively is the point:
/// a runtime added later is a compile error at every place the two differ,
/// rather than a silent fall through to whichever arm came first.
pub enum Runtime {
    Mono(MonoRuntime),
    Il2Cpp(Il2CppRuntime),
}

/// The low bits of the class kind byte, whose value 3 marks a generic
/// instance.
const CLASS_KIND_MASK: u8 = 0x7;
const GENERIC_INSTANCE_KIND: u8 = 3;

/// The element kinds a type's kind byte carries that the type route walks
/// through: a single-dimensional array's data leads on toward its element,
/// and a generic instance's data is the instantiation descriptor.
const SZARRAY: u8 = 0x1D;
const GENERIC_INSTANCE: u8 = 0x15;

/// Whether a type of this element kind names a class at all. End, Void,
/// Ptr, ByRef, Var, multidimensional Array, FnPtr, and MVar do not.
const fn names_a_class(kind: u8) -> bool {
    !matches!(kind, 0x00 | 0x01 | 0x0F | 0x10 | 0x13 | 0x14 | 0x1B | 0x1E)
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
    pub class_kind: Option<u16>,
    pub generic_class: Option<u16>,
    pub container_class: Option<u16>,
    pub type_data: Option<u16>,
    pub type_kind: Option<u16>,
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
    pub cached_class: Option<u16>,
    pub type_data: Option<u16>,
    pub type_kind: Option<u16>,
}

impl MonoRuntime {
    // The class whose count slot holds this class's count: a generic instance
    // carries the inflated fields itself but no count, so the definition it
    // was made from answers, reached through the instantiation descriptor.
    fn counted_class(
        &self,
        process: &Process,
        pointer_size: PointerSize,
        class: ClassRef,
    ) -> ClassRef {
        let (Some(class_kind), Some(generic_class), Some(container_class)) =
            (self.class_kind, self.generic_class, self.container_class)
        else {
            return class;
        };

        let kind = process
            .read::<u8>(class.address + class_kind)
            .unwrap_or_default();
        if kind & CLASS_KIND_MASK != GENERIC_INSTANCE_KIND {
            return class;
        }

        process
            .read_pointer(class.address + generic_class, pointer_size)
            .ok()
            .filter(|address| !address.is_null())
            .and_then(|descriptor| {
                process
                    .read_pointer(descriptor + container_class, pointer_size)
                    .ok()
            })
            .filter(|address| !address.is_null())
            .map_or(class, ClassRef::new)
    }
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
    pub fn field_count(
        &self,
        process: &Process,
        pointer_size: PointerSize,
        class: ClassRef,
    ) -> u64 {
        match self {
            Self::Mono(mono) => process
                .read::<i32>(
                    mono.counted_class(process, pointer_size, class).address + mono.field_count,
                )
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

    /// Resolves the class a field's type names, for the kinds a collection's
    /// backing field presents: an array of a class the runtimes already
    /// inflated. Kinds that name no class, and the table-resolved plain
    /// definitions IL2CPP keeps behind an index or a handle, answer nothing.
    pub fn class_from_type(
        &self,
        process: &Process,
        pointer_size: PointerSize,
        type_address: Address,
    ) -> Option<ClassRef> {
        match self {
            // Mono's data names the class itself, an array's element class
            // included. A generic instance's data is the instantiation
            // descriptor rather than a class, and no collections trace
            // presents one here: the array hop already landed on the
            // inflated class.
            Self::Mono(mono) => {
                let (data, kind) = (mono.type_data?, mono.type_kind?);
                let kind = process.read::<u8>(type_address + kind).ok()?;
                if !names_a_class(kind) || kind == GENERIC_INSTANCE {
                    return None;
                }

                process
                    .read_pointer(type_address + data, pointer_size)
                    .ok()
                    .filter(|address| !address.is_null())
                    .map(ClassRef::new)
            }
            // IL2CPP's array data is the element's own type, walked onward;
            // a generic instance's descriptor caches the class it resolved
            // to. Deeper array nesting than this is garbage.
            Self::Il2Cpp(il2cpp) => {
                let (data, kind, cached) =
                    (il2cpp.type_data?, il2cpp.type_kind?, il2cpp.cached_class?);

                let mut at = type_address;
                for _ in 0..8 {
                    let element = process.read::<u8>(at + kind).ok()?;
                    if !names_a_class(element) {
                        return None;
                    }

                    let data = process
                        .read_pointer(at + data, pointer_size)
                        .ok()
                        .filter(|address| !address.is_null())?;

                    match element {
                        SZARRAY => at = data,
                        GENERIC_INSTANCE => {
                            return process
                                .read_pointer(data + cached, pointer_size)
                                .ok()
                                .filter(|address| !address.is_null())
                                .map(ClassRef::new)
                        }
                        _ => return None,
                    }
                }

                None
            }
        }
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
