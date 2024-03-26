//! Support for games using the Unreal Engine

use core::{
    array,
    cell::RefCell,
    iter::{self, FusedIterator},
    mem::size_of,
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
    //version: Version,
    offsets: &'static Offsets,
    g_engine: Address,
    g_world: Address,
    fname_base: Address,
}

impl Module {
    /// Tries attaching to a UE4 game. The UE version needs to be correct for this
    /// function to work.
    pub fn attach(
        process: &Process,
        version: Version,
        main_module_address: Address,
    ) -> Option<Self> {
        let pointer_size = pe::MachineType::read(process, main_module_address)?.pointer_size()?;
        let offsets = Offsets::new(version, pointer_size)?;
        let module_size = pe::read_size_of_image(process, main_module_address)? as u64;
        let module_range = (main_module_address, module_size);

        let g_engine = {
            const GENGINE_1: Signature<7> = Signature::new("A8 01 75 ?? 48 C7 05");
            const GENGINE_2: Signature<6> = Signature::new("A8 01 75 ?? C7 05");

            if let Some(val) =
                GENGINE_1.scan_process_range(process, module_range)
            {
                let val = val + 0x7;
                val + 0x8 + process.read::<i32>(val).ok()?
            } else {
                let val =
                    GENGINE_2.scan_process_range(process, module_range)?;
                let val = val + 0x6;
                val + 0x8 + process.read::<i32>(val).ok()?
            }
        };

        let g_world = {
            const GWORLD: Signature<22> =
                Signature::new("48 8B 05 ?? ?? ?? ?? 48 3B ?? 48 0F 44 ?? 48 89 05 ?? ?? ?? ?? E8");
            let val = GWORLD.scan_process_range(process, module_range)? + 3;
            val + 0x4 + process.read::<i32>(val).ok()?
        };

        let fname_base = {
            const FNAME_POOL_0: Signature<11> = Signature::new("74 09 48 8D 15 ?? ?? ?? ?? EB 16");
            const FNAME_POOL_1: Signature<13> =
                Signature::new("89 5C 24 ?? 89 44 24 ?? 74 ?? 48 8D 15");
            const FNAME_POOL_2: Signature<13> =
                Signature::new("57 0F B7 F8 74 ?? B8 ?? ?? ?? ?? 8B 44");

            if let Some(scan) =
                FNAME_POOL_0.scan_process_range(process, module_range)
            {
                let val = scan + 5;
                val + 0x4 + process.read::<i32>(val).ok()?
            } else if let Some(scan) =
                FNAME_POOL_1.scan_process_range(process, module_range)
            {
                let val = scan + 13;
                val + 0x4 + process.read::<i32>(val).ok()?
            } else if let Some(scan) =
                FNAME_POOL_2.scan_process_range(process, module_range)
            {
                let val = scan + 7;
                val + 0x4 + process.read::<i32>(val).ok()?
            } else {
                None?
            }
        };

        Some(Self {
            pointer_size,
            //version,
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
        match process.read_pointer(self.g_world, self.pointer_size) {
            Ok(Address::NULL) | Err(_) => None,
            Ok(val) => Some(UObject { object: val }),
        }
    }

    /// Returns the current instance of GEngine
    pub fn get_gengine_uobject(&self, process: &Process) -> Option<UObject> {
        match process.read_pointer(self.g_engine, self.pointer_size) {
            Ok(Address::NULL) | Err(_) => None,
            Ok(val) => Some(UObject { object: val }),
        }
    }

    #[inline]
    const fn size_of_ptr(&self) -> u64 {
        self.pointer_size as u64
    }
}

#[allow(missing_docs)]
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
        )? + (name_offset as u64).wrapping_mul(size_of::<u16>() as u64);

        let string_size = process
            .read::<u16>(addr)?
            .checked_shr(6)
            .unwrap_or_default() as usize;

        let mut string = process.read::<ArrayCString<N>>(addr + size_of::<u16>() as u64)?;
        string.set_len(string_size);

        Ok(string)
    }

    fn get_uclass(&self, process: &Process, module: &Module) -> Result<UClass, Error> {
        match process.read_pointer(
            self.object + module.offsets.uobject_class,
            module.pointer_size,
        ) {
            Ok(Address::NULL) | Err(_) => Err(Error {}),
            Ok(val) => Ok(UClass { class: val }),
        }
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
    fn properties<'a>(
        &'a self,
        process: &'a Process,
        module: &'a Module,
    ) -> impl FusedIterator<Item = UProperty> + '_ {
        let mut current_property = {
            let mut val = None;
            let mut current_class = *self;

            while val.is_none() {
                if let Ok(current_property_address) = process.read_pointer(
                    current_class.class + module.offsets.uclass_property_link,
                    module.pointer_size,
                ) {
                    if current_property_address.is_null() {
                        if let Ok(super_field) = process.read_pointer(
                            current_class.class + module.offsets.uclass_super_field,
                            module.pointer_size,
                        ) {
                            if super_field.is_null() {
                                break;
                            } else {
                                current_class = UClass { class: super_field };
                            }
                        }
                    } else {
                        val = Some(UProperty {
                            property: current_property_address,
                        });
                    }
                } else {
                    break;
                }
            }

            val
        };

        iter::from_fn(move || {
            if let Some(prop) = current_property {
                current_property = {
                    if let Ok(val) = process.read_pointer(
                        prop.property + module.offsets.uproperty_property_link_next,
                        module.pointer_size,
                    ) {
                        Some(UProperty { property: val })
                    } else {
                        None
                    }
                };

                Some(prop)
            } else {
                None
            }
        })
        .fuse()
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
        let [name_offset, chunk_offset] =
            process.read::<[u16; 2]>(self.property + module.offsets.uproperty_fname)?;

        let addr = process.read_pointer(
            module.fname_base + module.size_of_ptr().wrapping_mul(chunk_offset as u64 + 2),
            module.pointer_size,
        )? + (name_offset as u64).wrapping_mul(size_of::<u16>() as u64);

        let string_size = process
            .read::<u16>(addr)?
            .checked_shr(6)
            .unwrap_or_default() as usize;

        let mut string = process.read::<ArrayCString<N>>(addr + size_of::<u16>() as u64)?;
        string.set_len(string_size);

        Ok(string)
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

        let mut current_uobject = UObject {
            object: match cache.resolved_offsets {
                0 => process.read_pointer(self.base_address, module.pointer_size)?,
                x => {
                    let mut addr = process.read_pointer(self.base_address, module.pointer_size)?;
                    for &i in &cache.offsets[..x] {
                        addr = process.read_pointer(addr + i, module.pointer_size)?;
                    }
                    addr
                }
            },
        };

        for i in cache.resolved_offsets..self.depth {
            let offset_from_string = match self.fields[i].strip_prefix("0x") {
                Some(rem) => u32::from_str_radix(rem, 16).ok(),
                _ => self.fields[i].parse().ok(),
            };

            let current_offset = match offset_from_string {
                Some(offset) => offset as u64,
                _ => current_uobject
                    .get_field_offset(process, module, self.fields[i])
                    .ok_or(Error {})? as u64,
            };

            cache.offsets[i] = current_offset;
            cache.resolved_offsets += 1;

            current_uobject = UObject {
                object: process
                    .read_pointer(current_uobject.object + current_offset, module.pointer_size)?,
            };
        }
        Ok(())
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
    uproperty_fname: u8,
    uproperty_offset_internal: u8,
    uproperty_property_link_next: u8,
}

impl Offsets {
    const fn new(version: Version, pointer_size: PointerSize) -> Option<&'static Self> {
        match pointer_size {
            PointerSize::Bit64 => Some(match version {
                // Tested on: Sonic Omens
                Version::V4_23 | Version::V4_24 => &Self {
                    uobject_fname: 0x18,
                    uobject_class: 0x10,
                    uclass_super_field: 0x40,
                    uclass_property_link: 0x48,
                    uproperty_fname: 0x18,
                    uproperty_offset_internal: 0x44,
                    uproperty_property_link_next: 0x50,
                },
                // Tested on: Tetris Effect / Kao the Kangaroo
                Version::V4_25
                | Version::V4_26
                | Version::V4_27
                | Version::V5_0
                | Version::V5_1
                | Version::V5_2 => &Self {
                    uobject_fname: 0x18,
                    uobject_class: 0x10,
                    uclass_super_field: 0x40,
                    uclass_property_link: 0x50,
                    uproperty_fname: 0x28,
                    uproperty_offset_internal: 0x4C,
                    uproperty_property_link_next: 0x58,
                },
                // Tested on Unreal Physics
                Version::V5_3 => &Self {
                    uobject_fname: 0x18,
                    uobject_class: 0x10,
                    uclass_super_field: 0x40,
                    uclass_property_link: 0x50,
                    uproperty_fname: 0x20,
                    uproperty_offset_internal: 0x44,
                    uproperty_property_link_next: 0x48,
                },
            }),
            _ => None,
        }
    }
}

#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Hash, Debug, PartialOrd)]
#[allow(missing_docs)]
/// The version of Unreal Engine used by the game
pub enum Version {
    V4_23,
    V4_24,
    V4_25,
    V4_26,
    V4_27,
    V5_0,
    V5_1,
    V5_2,
    V5_3,
}
