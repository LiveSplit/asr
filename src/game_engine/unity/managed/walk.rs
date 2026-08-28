use super::super::{get_backing_name, CSTR};
use super::{ClassRef, ClimbStop, FieldRef, ImageRef, Runtime, WalkOffsets};
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

            let field_count = self.runtime.field_count(process, class);

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
