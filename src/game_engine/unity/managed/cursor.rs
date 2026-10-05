use super::runtime::{Il2CppRuntime, MonoRuntime};
use super::{slot, ClassRef};
use crate::{cycle::Cycle, Address, Address32, Address64, PointerSize, Process};

/// The most assemblies an IL2CPP game can load. No game loads anywhere near
/// 65,536.
const ASSEMBLIES: u64 = 1 << 16;

/// Walks the runtime's loaded assemblies.
pub struct Assemblies<'a> {
    process: &'a Process,
    pointer_size: PointerSize,
    state: AssembliesState,
}

enum AssembliesState {
    /// The glib list: each node carries the assembly and the next node.
    Mono {
        node: Option<Address>,
        /// Where the list has been.
        cycle: Cycle,
    },
    /// The vector: a slice of assembly pointers.
    Il2Cpp {
        base: Address,
        count: u64,
        index: u64,
    },
}

impl<'a> Assemblies<'a> {
    pub(super) fn mono(
        process: &'a Process,
        pointer_size: PointerSize,
        mono: &MonoRuntime,
    ) -> Self {
        let node = process
            .read_pointer(mono.assemblies, pointer_size)
            .ok()
            .filter(|address| !address.is_null());

        Self {
            process,
            pointer_size,
            state: AssembliesState::Mono {
                node,
                cycle: Cycle::new(node.unwrap_or_default()),
            },
        }
    }

    pub(super) fn il2cpp(
        process: &'a Process,
        pointer_size: PointerSize,
        il2cpp: &Il2CppRuntime,
    ) -> Self {
        let first = process
            .read_pointer(il2cpp.assemblies, pointer_size)
            .unwrap_or_default();
        let limit = process
            .read_pointer(il2cpp.assemblies + pointer_size as u64, pointer_size)
            .unwrap_or_default();

        Self {
            process,
            pointer_size,
            state: AssembliesState::Il2Cpp {
                base: first,
                // A count past ASSEMBLIES comes from wrong offsets and reads
                // as no assemblies.
                count: Some(limit.value().saturating_sub(first.value()) / pointer_size as u64)
                    .filter(|&count| count <= ASSEMBLIES)
                    .unwrap_or_default(),
                index: 0,
            },
        }
    }
}

impl Iterator for Assemblies<'_> {
    type Item = Address;

    fn next(&mut self) -> Option<Address> {
        match &mut self.state {
            AssembliesState::Mono { node, cycle } => {
                let at = (*node)?;

                let [data, next]: [Address; 2] = match self.pointer_size {
                    PointerSize::Bit64 => self
                        .process
                        .read::<[Address64; 2]>(at)
                        .ok()?
                        .map(|address| address.into()),
                    _ => self
                        .process
                        .read::<[Address32; 2]>(at)
                        .ok()?
                        .map(|address| address.into()),
                };

                *node = Some(next).filter(|&next| !cycle.revisits(next));

                Some(data)
            }
            AssembliesState::Il2Cpp { base, count, index } => loop {
                if index >= count {
                    return None;
                }

                let at = slot(*base, self.pointer_size, *index);
                *index += 1;

                if let Some(assembly) = self
                    .process
                    .read_pointer(at, self.pointer_size)
                    .ok()
                    .filter(|address| !address.is_null())
                {
                    return Some(assembly);
                }
            },
        }
    }
}

/// The most buckets an image's class table can have. Mono sizes the table by
/// the number of classes, and no image holds anywhere near 1,000,000.
const BUCKETS: u32 = 1 << 20;

/// The most types an IL2CPP image can hold. No image holds anywhere near
/// 1,000,000.
const TYPES: u32 = 1 << 20;

/// Walks the classes an image holds.
pub struct Classes<'a> {
    process: &'a Process,
    pointer_size: PointerSize,
    state: ClassesState,
}

enum ClassesState {
    /// The image's hash table: a bucket array whose entries chain through the
    /// classes themselves.
    Mono {
        table: Address,
        size: u64,
        bucket: u64,
        chain: Option<Address>,
        /// Where the current bucket's chain has been.
        cycle: Cycle,
        next_class_cache: u16,
    },
    /// The image's slice of the type info definition table.
    Il2Cpp {
        slots: Address,
        count: u64,
        index: u64,
    },
}

impl<'a> Classes<'a> {
    pub(super) fn mono(
        process: &'a Process,
        pointer_size: PointerSize,
        mono: &MonoRuntime,
        image: super::ImageRef,
    ) -> Self {
        let cache = image.address + mono.class_cache;

        // The size the runtime stores is signed. A negative size, or one past
        // BUCKETS, comes from wrong offsets and reads as an empty table.
        let size = process
            .read::<i32>(cache + mono.hash_table_size)
            .ok()
            .and_then(|size| u32::try_from(size).ok())
            .filter(|&size| size <= BUCKETS)
            .unwrap_or_default() as u64;

        let table = match size {
            0 => Address::NULL,
            _ => process
                .read_pointer(cache + mono.hash_table_table, pointer_size)
                .unwrap_or_default(),
        };

        Self {
            process,
            pointer_size,
            state: ClassesState::Mono {
                table,
                size,
                bucket: 0,
                chain: None,
                cycle: Cycle::new(Address::NULL),
                next_class_cache: mono.next_class_cache,
            },
        }
    }

    pub(super) fn il2cpp(
        process: &'a Process,
        pointer_size: PointerSize,
        il2cpp: &Il2CppRuntime,
        image: super::ImageRef,
    ) -> Self {
        // A count past TYPES comes from wrong offsets and reads as an image
        // with no classes.
        let count = process
            .read::<u32>(image.address + il2cpp.type_count)
            .ok()
            .filter(|&count| count <= TYPES)
            .unwrap_or_default() as u64;

        let metadata = match (count, il2cpp.handle_is_inline) {
            (0, _) => Address::NULL,
            (_, true) => image.address + il2cpp.metadata_handle,
            (_, false) => process
                .read_pointer(image.address + il2cpp.metadata_handle, pointer_size)
                .unwrap_or_default(),
        };

        let handle = match metadata {
            Address::NULL => 0,
            at => process.read::<u32>(at).unwrap_or_default(),
        };

        let table = match metadata {
            Address::NULL => Address::NULL,
            _ => process
                .read_pointer(il2cpp.type_info_definition_table, pointer_size)
                .unwrap_or_default(),
        };

        let slots = match table {
            Address::NULL => Address::NULL,
            _ => slot(table, pointer_size, handle as u64),
        };

        Self {
            process,
            pointer_size,
            state: ClassesState::Il2Cpp {
                slots,
                count,
                index: 0,
            },
        }
    }

    /// The slot array and its length, for the caller that iterates the slots
    /// itself.
    pub const fn slots(&self) -> Option<(Address, u64)> {
        match &self.state {
            ClassesState::Il2Cpp { slots, count, .. } => Some((*slots, *count)),
            ClassesState::Mono { .. } => None,
        }
    }
}

impl Iterator for Classes<'_> {
    type Item = ClassRef;

    fn next(&mut self) -> Option<ClassRef> {
        match &mut self.state {
            ClassesState::Mono {
                table,
                size,
                bucket,
                chain,
                cycle,
                next_class_cache,
            } => loop {
                if let Some(class) = *chain {
                    *chain = self
                        .process
                        .read_pointer(class + *next_class_cache, self.pointer_size)
                        .ok()
                        .filter(|&address| !address.is_null() && !cycle.revisits(address));

                    return Some(ClassRef::new(class));
                }

                if table.is_null() || bucket >= size {
                    return None;
                }

                *chain = self
                    .process
                    .read_pointer(slot(*table, self.pointer_size, *bucket), self.pointer_size)
                    .ok()
                    .filter(|address| !address.is_null());
                *cycle = Cycle::new(chain.unwrap_or_default());
                *bucket += 1;
            },
            ClassesState::Il2Cpp {
                slots,
                count,
                index,
            } => loop {
                if index >= count {
                    return None;
                }

                let at = slot(*slots, self.pointer_size, *index);
                *index += 1;

                if let Some(class) = self
                    .process
                    .read_pointer(at, self.pointer_size)
                    .ok()
                    .filter(|address| !address.is_null())
                {
                    return Some(ClassRef::new(class));
                }
            },
        }
    }
}
