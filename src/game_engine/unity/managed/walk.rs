use super::super::{get_backing_name, CSTR};
use super::readers::ENTRY_SCRATCH;
use super::{
    ClassRef, ClimbStop, DictionaryOffsets, EntryLayout, FieldRef, ImageRef, ListOffsets, Runtime,
    WalkOffsets,
};
use crate::{string::ArrayCString, Address, PointerSize, Process};

/// The walk itself: everything both runtimes lay out the same way, written
/// once against the operations [`Runtime`] supplies. An adapter builds one per
/// call from what its module holds, so nothing here is stored anywhere.
pub struct Walk {
    pub runtime: Runtime,
    pub offsets: WalkOffsets,
    pub stop: ClimbStop,
    pub pointer_size: PointerSize,
}

impl Walk {
    /// Reads an assembly's name, off the assembly itself or through its image,
    /// whichever the offsets carry.
    pub fn assembly_name<const N: usize>(
        &self,
        process: &Process,
        assembly: Address,
    ) -> Option<ArrayCString<N>> {
        let assembly_offsets = &self.offsets.assembly;

        let at = match (
            assembly_offsets.name_in_image,
            assembly_offsets.name_in_assembly,
        ) {
            (Some(name), _) => self.assembly_image(process, assembly)?.address + name,
            (_, Some(name)) => assembly + name,
            _ => return None,
        };

        super::read_name(process, self.pointer_size, at)
    }

    /// Reads the image an assembly carries.
    pub fn assembly_image(&self, process: &Process, assembly: Address) -> Option<ImageRef> {
        process
            .read_pointer(assembly + self.offsets.assembly.image, self.pointer_size)
            .ok()
            .filter(|address| !address.is_null())
            .map(ImageRef::new)
    }

    pub fn class_name<const N: usize>(
        &self,
        process: &Process,
        class: ClassRef,
    ) -> Option<ArrayCString<N>> {
        super::read_name(
            process,
            self.pointer_size,
            class.address + self.offsets.class.name,
        )
    }

    pub fn class_namespace<const N: usize>(
        &self,
        process: &Process,
        class: ClassRef,
    ) -> Option<ArrayCString<N>> {
        super::read_name(
            process,
            self.pointer_size,
            class.address + self.offsets.class.namespace,
        )
    }

    /// Resolves a loaded image by its assembly name.
    pub fn find_image(&self, process: &Process, name: &str) -> Option<ImageRef> {
        self.runtime
            .assemblies(process, self.pointer_size)
            .find(|&assembly| {
                self.assembly_name::<CSTR>(process, assembly)
                    .is_some_and(|read| read.matches(name))
            })
            .and_then(|assembly| self.assembly_image(process, assembly))
    }

    /// Resolves a class by name, with the namespace split off at the last dot
    /// when one is written. A nested class is written the way .NET writes it,
    /// `Outer+Inner`: the runtime stores the innermost name bare, so the leaf
    /// is what the lookup matches on, and the written enclosure is checked by
    /// climbing.
    pub fn find_class(
        &self,
        process: &Process,
        image: ImageRef,
        class_name: &str,
    ) -> Option<ClassRef> {
        if let Some(plus) = class_name.find('+') {
            let name_space_index = class_name[..plus].rfind('.');
            let (name_space, nested) = match name_space_index {
                Some(index) => (&class_name[..index], &class_name[index + 1..]),
                None => ("", class_name),
            };

            // Never measured where a class keeps its enclosing class means the
            // written enclosure cannot be checked, and an unchecked leaf match
            // would be a guess.
            let declaring = self.offsets.class.declaring?;
            let leaf = nested.rsplit('+').next()?;

            return self
                .runtime
                .classes(process, self.pointer_size, image)
                .find(|&class| {
                    self.class_name::<CSTR>(process, class)
                        .is_some_and(|name| name.matches(leaf))
                        && self.encloses(process, class, nested, name_space, declaring)
                });
        }

        let name_space_index = class_name.rfind('.');

        self.runtime
            .classes(process, self.pointer_size, image)
            .find(|&class| {
                self.class_name::<CSTR>(process, class).is_some_and(|name| {
                    if let Some(name_space_index) = name_space_index {
                        let class_name_space = &class_name[..name_space_index];
                        let class_name = &class_name[name_space_index + 1..];

                        name.matches(class_name)
                            && self
                                .class_namespace::<CSTR>(process, class)
                                .is_some_and(|name_space| name_space.matches(class_name_space))
                    } else {
                        name.matches(class_name)
                    }
                })
            })
    }

    // Whether a class whose own name matched the leaf is the one the written
    // name meant: each step out has to be the part written before it, the
    // outermost has to be enclosed by nothing, and the namespace belongs to
    // the outermost, the leaf's own being empty when nested.
    fn encloses(
        &self,
        process: &Process,
        class: ClassRef,
        nested: &str,
        name_space: &str,
        declaring: u16,
    ) -> bool {
        let enclosing = |class: ClassRef| {
            process
                .read_pointer(class.address + declaring, self.pointer_size)
                .ok()
        };

        let mut outer = class;
        for part in nested.rsplit('+').skip(1) {
            let Some(address) = enclosing(outer).filter(|address| !address.is_null()) else {
                return false;
            };
            outer = ClassRef::new(address);

            if !self
                .class_name::<CSTR>(process, outer)
                .is_some_and(|name| name.matches(part))
            {
                return false;
            }
        }

        if !enclosing(outer).is_some_and(|address| address.is_null()) {
            return false;
        }

        self.class_namespace::<CSTR>(process, outer)
            .is_some_and(|read| read.matches(name_space))
    }

    /// Resolves the parent class.
    pub fn parent(&self, process: &Process, class: ClassRef) -> Option<ClassRef> {
        process
            .read_pointer(class.address + self.offsets.class.parent, self.pointer_size)
            .ok()
            .filter(|address| !address.is_null())
            .map(ClassRef::new)
    }

    /// Resolves a field by name, climbing the parent chain until either stop
    /// name answers, matching the written name or its backing field. Hands
    /// back the class the field was found on as well as the offset, since a
    /// static field's offset measures into that class's own static table.
    pub fn find_field_offset(
        &self,
        process: &Process,
        class: ClassRef,
        field_name: &str,
    ) -> Option<(ClassRef, u32)> {
        let mut this_class = Some(class);

        loop {
            let class = this_class?;

            if self
                .class_name::<CSTR>(process, class)?
                .matches(self.stop.class)
                || self
                    .class_namespace::<CSTR>(process, class)?
                    .matches(self.stop.namespace)
            {
                return None;
            }

            this_class = self.parent(process, class);

            let field_count = self.runtime.field_count(process, self.pointer_size, class);

            let fields = match field_count {
                0 => None,
                _ => process
                    .read_pointer(class.address + self.offsets.class.fields, self.pointer_size)
                    .ok()
                    .filter(|address| !address.is_null()),
            };

            let Some(fields) = fields else {
                continue;
            };

            for index in 0..field_count {
                let field =
                    FieldRef::new(fields + index.wrapping_mul(self.offsets.field.stride as u64));

                let matched = self.field_name::<CSTR>(process, field).is_some_and(|name| {
                    name.matches(field_name)
                        || name
                            .validate_utf8()
                            .ok()
                            .and_then(get_backing_name)
                            .is_some_and(|name| name == field_name)
                });

                if matched {
                    return Some((class, self.field_offset(process, field)?));
                }
            }
        }
    }

    fn field_name<const N: usize>(
        &self,
        process: &Process,
        field: FieldRef,
    ) -> Option<ArrayCString<N>> {
        super::read_name(
            process,
            self.pointer_size,
            field.address + self.offsets.field.name,
        )
    }

    fn field_offset(&self, process: &Process, field: FieldRef) -> Option<u32> {
        process.read(field.address + self.offsets.field.offset).ok()
    }

    /// Resolves where a list keeps its backing array and live count, off the
    /// `System.Collections.Generic.List` class in the object's parent chain.
    /// Corlib names both fields the same across every generation the offsets
    /// tables cover. This also supports classes derived from `List` without
    /// accepting an unrelated class that happens to use the same field names.
    pub fn list_offsets(&self, process: &Process, object: Address) -> Option<ListOffsets> {
        let mut class = self.object_class(process, object)?;

        loop {
            if self.class_name::<CSTR>(process, class)?.matches("List`1")
                && self
                    .class_namespace::<CSTR>(process, class)?
                    .matches("System.Collections.Generic")
            {
                break;
            }

            class = self.parent(process, class)?;
        }

        let field_count = self.runtime.field_count(process, self.pointer_size, class);
        let fields = process
            .read_pointer(class.address + self.offsets.class.fields, self.pointer_size)
            .ok()
            .filter(|address| !address.is_null())?;

        let mut items = None;
        let mut size = None;
        for index in 0..field_count {
            let field =
                FieldRef::new(fields + index.wrapping_mul(self.offsets.field.stride as u64));

            let Some(name) = self.field_name::<CSTR>(process, field) else {
                continue;
            };

            if name.matches("_items") {
                items = self.field_offset(process, field);
            } else if name.matches("_size") {
                size = self.field_offset(process, field);
            }
        }

        Some(ListOffsets {
            items: items?,
            size: size?,
        })
    }

    /// Resolves where a dictionary keeps its backing entries and live
    /// counts, and how one entry lays out, off the
    /// `System.Collections.Generic.Dictionary` class in the object's parent
    /// chain. Both corlib naming generations answer; the buckets field has
    /// to exist for the shape to be this one, though nothing here reads it.
    /// The parallel-arrays shape the oldest corlib used carries other names
    /// and misses cleanly.
    pub fn dictionary_offsets(
        &self,
        process: &Process,
        object: Address,
    ) -> Option<DictionaryOffsets> {
        let mut class = self.object_class(process, object)?;

        loop {
            if self
                .class_name::<CSTR>(process, class)?
                .matches("Dictionary`2")
                && self
                    .class_namespace::<CSTR>(process, class)?
                    .matches("System.Collections.Generic")
            {
                break;
            }

            class = self.parent(process, class)?;
        }

        let mut found = [None; 4];
        self.each_own_field(process, class, |name, field, offset| {
            let generations = [
                ["_buckets", "_entries", "_count", "_freeCount"],
                ["buckets", "entries", "count", "freeCount"],
            ];
            for generation in generations {
                for (slot, member) in generation.into_iter().enumerate() {
                    if name.matches(member) {
                        found[slot] = Some((field, offset));
                    }
                }
            }
        });
        let (Some(_), Some(entries), Some(count), Some(free_count)) =
            (found[0], found[1], found[2], found[3])
        else {
            return None;
        };

        // The entry class arrives through the entries field's own type, and
        // its instance size, boxed header removed, is the entry stride.
        let entry_type = process
            .read_pointer(
                entries.0.address + self.offsets.field.type_?,
                self.pointer_size,
            )
            .ok()
            .filter(|address| !address.is_null())?;
        let entry_class = self
            .runtime
            .class_from_type(process, self.pointer_size, entry_type)?;

        let header = 2 * self.pointer_size as u32;
        let size = process
            .read::<i32>(entry_class.address + self.offsets.class.instance_size?)
            .ok()?;
        let stride = u32::try_from(size)
            .ok()?
            .checked_sub(header)
            .filter(|&stride| stride > 0 && stride as usize <= ENTRY_SCRATCH)?;

        // The members' offsets are recorded as if boxed; folding them by the
        // header measures them from the entry's own start.
        let mut members = [None; 4];
        self.each_own_field(process, entry_class, |name, _, offset| {
            for (slot, member) in ["hashCode", "next", "key", "value"].into_iter().enumerate() {
                if name.matches(member) {
                    members[slot] = offset.checked_sub(header);
                }
            }
        });
        let (Some(hash), Some(next), Some(key), Some(value)) =
            (members[0], members[1], members[2], members[3])
        else {
            return None;
        };

        let ends_inside = |member: u32| member.checked_add(4).is_some_and(|end| end <= stride);
        let holds = ends_inside(hash) && ends_inside(next) && key < stride && value < stride;
        holds.then_some(DictionaryOffsets {
            entries: entries.1,
            count: count.1,
            free_count: free_count.1,
            layout: EntryLayout {
                stride,
                hash,
                next,
                key,
                value,
            },
        })
    }

    // Hands every field the class itself declares to the callback, with its
    // name and instance offset. Unreadable entries are skipped, the way the
    // field climb skips them.
    fn each_own_field(
        &self,
        process: &Process,
        class: ClassRef,
        mut each: impl FnMut(&ArrayCString<CSTR>, FieldRef, u32),
    ) {
        let field_count = self.runtime.field_count(process, self.pointer_size, class);
        let Some(fields) = process
            .read_pointer(class.address + self.offsets.class.fields, self.pointer_size)
            .ok()
            .filter(|address| !address.is_null())
        else {
            return;
        };

        for index in 0..field_count {
            let field =
                FieldRef::new(fields + index.wrapping_mul(self.offsets.field.stride as u64));

            let Some(name) = self.field_name::<CSTR>(process, field) else {
                continue;
            };
            let Some(offset) = self.field_offset(process, field) else {
                continue;
            };

            each(&name, field, offset);
        }
    }

    /// Reads the address a class's static field offsets are measured from.
    pub fn static_table(&self, process: &Process, class: ClassRef) -> Option<Address> {
        self.runtime.static_table(process, self.pointer_size, class)
    }

    /// Reads the class a live object belongs to.
    pub fn object_class(&self, process: &Process, object: Address) -> Option<ClassRef> {
        self.runtime
            .object_class(process, self.pointer_size, object)
    }
}
