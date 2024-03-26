use core::{
    array,
    cell::RefCell,
    iter::{self, FusedIterator},
};

use bytemuck::CheckedBitPattern;

use crate::{
    file_format::pe, signature::Signature, string::ArrayCString, Address, Error, PointerSize,
    Process,
};

const CSTR: usize = 128;

/// Represents access to a Unreal Engine game
pub struct Module {
    pointer_size: PointerSize,
    version: Version,
    offsets: &'static Offsets,
    g_engine: Address,
    g_world: Address,
    fname_base: Address,
}

impl Module {
    /// Tries attaching to a UE4 game. The UE version needs to be correct for this
    /// function to work. If you don't know the version in advance, use
    /// [`attach_auto_detect`](Self::attach_auto_detect) instead
    pub fn attach(
        process: &Process,
        version: Version,
        main_module_address: Address,
    ) -> Option<Self> {
        let pointer_size = pe::MachineType::read(process, main_module_address)?.pointer_size()?;
        let offsets = Offsets::new(version, pointer_size)?;

        let module_size = pe::read_size_of_image(process, main_module_address)? as u64;

        let g_engine = {
            const GENGINE_1: Signature<7> = Signature::new("A8 01 75 ?? 48 C7 05");
            const GENGINE_2: Signature<6> = Signature::new("A8 01 75 ?? C7 05");

            if let Some(val) =
                GENGINE_1.scan_process_range(process, (main_module_address, module_size))
            {
                let val = val + 0x7;
                val + 0x8 + process.read::<i32>(val).ok()?
            } else {
                let val =
                    GENGINE_2.scan_process_range(process, (main_module_address, module_size))?;
                let val = val + 0x6;
                val + 0x8 + process.read::<i32>(val).ok()?
            }
        };

        let g_world = {
            const GWORLD: Signature<22> =
                Signature::new("48 8B 05 ?? ?? ?? ?? 48 3B ?? 48 0F 44 ?? 48 89 05 ?? ?? ?? ?? E8");
            let val = GWORLD.scan_process_range(process, (main_module_address, module_size))? + 3;
            val + 0x4 + process.read::<i32>(val).ok()?
        };

        let fname_base = {
            const FNAME_POOL: Signature<11> = Signature::new("74 09 48 8D 15 ?? ?? ?? ?? EB 16");
            let val =
                FNAME_POOL.scan_process_range(process, (main_module_address, module_size))? + 5;
            val + 0x4 + process.read::<i32>(val).ok()?
        };

        Some(Self {
            pointer_size,
            version,
            offsets,
            g_engine,
            g_world,
            fname_base,
        })
    }

    /// Returns the memory pointer to GWorld
    pub fn gworld(&self) -> Address {
        self.g_world
    }

    /// Returns the memory pointer to GEngine
    pub fn gengine(&self) -> Address {
        self.g_engine
    }

    /// Returns the current instance of GWorld
    pub fn get_gworld_uobject(&self, process: &Process) -> Option<UObject> {
        Some(UObject {
            object: process.read_pointer(self.g_world, self.pointer_size).ok()?,
        })
    }

    /// Returns the current instance of GEngine
    pub fn get_gengine_uobject(&self, process: &Process) -> Option<UObject> {
        Some(UObject {
            object: process
                .read_pointer(self.g_engine, self.pointer_size)
                .ok()?,
        })
    }

    #[inline]
    const fn size_of_ptr(&self) -> u64 {
        self.pointer_size as u64
    }
}

#[derive(Copy, Clone)]
pub struct UObject {
    object: Address,
}

impl UObject {
    /// Reads the FName of the current UObject
    pub fn get_fname<const N: usize>(
        &self,
        process: &Process,
        module: &Module,
    ) -> Result<ArrayCString<N>, Error> {
        let [name_offset, chunk_offset] =
            process.read::<[u16; 2]>(self.object + module.offsets.uobject_fname)?;

        let addr = process.read_pointer(
            module.fname_base + module.size_of_ptr().wrapping_mul(chunk_offset as u64 + 2),
            module.pointer_size,
        )? + (name_offset as u64).wrapping_mul(2);
        let string_size = process
            .read::<u16>(addr)?
            .checked_shr(6)
            .unwrap_or_default() as usize;
        let mut string = process.read::<ArrayCString<N>>(addr + 2)?;
        string.set_len(string_size);
        Ok(string)
    }

    fn get_uclass(&self, process: &Process, module: &Module) -> Result<UClass, Error> {
        Ok(UClass {
            class: process.read_pointer(
                self.object + module.offsets.uobject_class,
                module.pointer_size,
            )?,
        })
    }

    /// Tries to find a field with the specified name in the current UObject and returns
    /// the offset of the field from the start of an instance of the class.
    pub fn get_field_offset(
        &self,
        process: &Process,
        module: &Module,
        field_name: &str,
    ) -> Option<u32> {
        self.get_uclass(process, module)
            .ok()?
            .get_field_offset(process, module, field_name)
    }
}

/// The UClass relative to a specific UObject
#[derive(Copy, Clone)]
struct UClass {
    class: Address,
}

impl UClass {
    fn get_parent(&self, process: &Process, module: &Module) -> Result<Self, Error> {
        Ok(UClass {
            class: process.read_pointer(
                self.class + module.offsets.uclass_super_field,
                module.pointer_size,
            )?,
        })
    }

    fn properties<'a>(
        &'a self,
        process: &'a Process,
        module: &'a Module,
    ) -> impl FusedIterator<Item = UProperty> + '_ {
        let mut this_class = *self;

        let mut current_property = UProperty {
            property: process
                .read_pointer(
                    self.class + module.offsets.uclass_property_link,
                    module.pointer_size,
                )
                .unwrap(),
        };

        iter::from_fn(move || {
            if this_class.class.is_null() {
                None
            } else {
                Some(
                    iter::from_fn(move || {
                        let prop = current_property;
                        if prop.property.is_null() {
                            this_class = this_class.get_parent(process, module).ok()?;
                            None
                        } else {
                            current_property = UProperty {
                                property: process
                                    .read_pointer(
                                        current_property.property
                                            + module.offsets.uproperty_property_link_next,
                                        module.pointer_size,
                                    )
                                    .unwrap_or_default(),
                            };
                            Some(prop)
                        }
                    })
                    .fuse(),
                )
            }
        })
        .fuse()
        .flatten()
    }

    fn get_field_offset(
        &self,
        process: &Process,
        module: &Module,
        field_name: &str,
    ) -> Option<u32> {
        self.properties(process, module)
            .find(|field| {
                field
                    .get_fname::<CSTR>(process, module)
                    .is_ok_and(|name| name.matches(field_name))
            })?
            .get_offset(process, module)
    }
}

#[derive(Copy, Clone)]
struct UProperty {
    property: Address,
}

impl UProperty {
    fn get_fname<const N: usize>(
        &self,
        process: &Process,
        module: &Module,
    ) -> Result<ArrayCString<N>, Error> {
        UObject {
            object: self.property,
        }
        .get_fname(process, module)
    }

    fn get_offset(&self, process: &Process, module: &Module) -> Option<u32> {
        process
            .read(self.property + module.offsets.uproperty_offset_internal)
            .ok()
    }
}

/// An implementation for automatic pointer path resolution
#[derive(Clone)]
pub struct UnrealPointer<const CAP: usize> {
    cache: RefCell<UnrealPointerCache<CAP>>,
    base_address: Address,
    fields: [&'static str; CAP],
    depth: usize,
}

#[derive(Clone, Copy)]
struct UnrealPointerCache<const CAP: usize> {
    offsets: [u64; CAP],
    resolved_offsets: usize,
    current_pointer: Address,
}

impl<const CAP: usize> UnrealPointer<CAP> {
    /// Creates a new instance of the Pointer struct
    ///
    /// `CAP` should be higher or equal to the number of offsets defined in `fields`.
    ///
    /// If a higher number of offsets is provided, the pointer path will be truncated
    /// according to the value of `CAP`.
    pub fn new(base_address: Address, fields: &[&'static str]) -> Self {
        let this_fields: [&str; CAP] = {
            let mut iter = fields.iter();
            array::from_fn(|_| iter.next().copied().unwrap_or_default())
        };

        let cache = RefCell::new(UnrealPointerCache {
            offsets: [u64::default(); CAP],
            resolved_offsets: usize::default(),
            current_pointer: Address::NULL,
        });

        Self {
            cache,
            base_address,
            fields: this_fields,
            depth: fields.len().min(CAP),
        }
    }

    /// Tries to resolve the pointer path
    fn find_offsets(&self, process: &Process, module: &Module) -> Result<(), Error> {
        let mut cache = self.cache.borrow_mut();

        // If the pointer path has already been found, there's no need to continue
        if cache.resolved_offsets == self.depth {
            return Ok(());
        }

        let mut current_pointer = match cache.current_pointer {
            Address::NULL => self.base_address,
            val => val,
        };

        for i in cache.resolved_offsets..self.depth {
            let uobject = UObject {
                object: process.read_pointer(current_pointer, module.pointer_size)?,
            };

            let offset_from_string = match self.fields[i].strip_prefix("0x") {
                Some(rem) => u32::from_str_radix(rem, 16).ok(),
                _ => self.fields[i].parse().ok(),
            };

            let current_offset = match offset_from_string {
                Some(offset) => offset as u64,
                _ => uobject
                    .get_field_offset(process, module, self.fields[i])
                    .ok_or(Error {})? as u64,
            };

            cache.offsets[i] = current_offset;
            current_pointer = uobject.object + current_offset;
            cache.current_pointer = current_pointer;
            cache.resolved_offsets += 1;
        }
        Ok(())
    }

    /// Dereferences the pointer path, returning the memory address of the value of interest
    pub fn deref_offsets(&self, process: &Process, module: &Module) -> Result<Address, Error> {
        self.find_offsets(process, module)?;
        let cache = self.cache.borrow();
        let mut address = process.read_pointer(self.base_address, module.pointer_size)?;
        let (&last, path) = cache.offsets[..self.depth].split_last().ok_or(Error {})?;
        for &offset in path {
            address = process.read_pointer(address + offset, module.pointer_size)?;
        }
        Ok(address + last)
    }

    /// Dereferences the pointer path, returning the value stored at the final memory address
    pub fn deref<T: CheckedBitPattern>(
        &self,
        process: &Process,
        module: &Module,
    ) -> Result<T, Error> {
        self.find_offsets(process, module)?;
        let cache = self.cache.borrow();
        process.read_pointer_path(
            process.read_pointer(self.base_address, module.pointer_size)?,
            module.pointer_size,
            &cache.offsets[..self.depth],
        )
    }
}

struct Offsets {
    uobject_fname: u8,
    uobject_class: u8,
    uclass_super_field: u8,
    uclass_property_link: u8,
    uproperty_offset_internal: u8,
    uproperty_property_link_next: u8,
}

impl Offsets {
    const fn new(version: Version, pointer_size: PointerSize) -> Option<&'static Self> {
        match pointer_size {
            PointerSize::Bit64 => Some(match version {
                _ => &Self {
                    uobject_fname: 0x18,
                    uobject_class: 0x10,
                    uclass_super_field: 0x28,
                    uclass_property_link: 0x48,
                    uproperty_offset_internal: 0x44,
                    uproperty_property_link_next: 0x50,
                },
            }),
            _ => None,
        }
    }
}

#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Hash, Debug)]
pub enum Version {
    V4_24,
    V4_25,
    V4_26,
    V4_27,
}
