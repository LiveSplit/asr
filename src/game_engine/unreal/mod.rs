//! Support for attaching to games using the Unreal Engine

use core::{
    array,
    cell::RefCell,
    iter::{self, FusedIterator},
    mem::size_of,
};

use bytemuck::CheckedBitPattern;

use crate::{
    file_format::pe,
    future::retry,
    signature::{Signature, SignatureScanner},
    string::ArrayCString,
    Address, Error, PointerSize, Process,
};

const CSTR: usize = 128;

/// Represents access to a Unreal Engine game.
///
/// This struct gives immediate access to 2 important structs present in every UE game:
/// - GEngine: a static object that persists throughout the process' lifetime
/// - GWorld: a pointer to the currently loaded UWorld object
pub struct Module {
    pointer_size: PointerSize,
    //version: Version,
    offsets: &'static Offsets,
    g_engine: Address,
    g_world: Address,
    fname_base: Address,
}

impl Module {
    /// Tries attaching to a UE game. The UE version needs to be correct for this
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
            const GENGINE: &[(Signature<7>, u8)] = &[
                (Signature::new("A8 01 75 ?? 48 C7 05"), 7),
                (Signature::new("A8 01 75 ?? C7 05 ??"), 6),
            ];

            let addr = GENGINE
                .iter()
                .find_map(|(sig, offset)| Some(sig.scan(process, module_range)? + *offset))?;
            addr + 0x8 + process.read::<i32>(addr).ok()?
        };

        let g_world = {
            const GWORLD: &[(Signature<22>, u8)] = &[
                (
                    Signature::new(
                        "80 7C 24 ?? 00 ?? ?? 48 8B 3D ?? ?? ?? ?? 48 ?? ?? ?? ?? ?? ?? ??",
                    ),
                    10,
                ),
                (
                    Signature::new(
                        "48 8B 05 ?? ?? ?? ?? 48 3B ?? 48 0F 44 ?? 48 89 05 ?? ?? ?? ?? E8",
                    ),
                    3,
                ),
            ];

            let addr = GWORLD
                .iter()
                .find_map(|(sig, offset)| Some(sig.scan(process, module_range)? + *offset))?;
            addr + 0x4 + process.read::<i32>(addr).ok()?
        };

        let fname_base = {
            const FNAME_POOL: &[(Signature<13>, u8)] = &[
                (Signature::new("74 09 48 8D 15 ?? ?? ?? ?? EB 16 ?? ??"), 5),
                (Signature::new("89 5C 24 ?? 89 44 24 ?? 74 ?? 48 8D 15"), 13),
                (Signature::new("57 0F B7 F8 74 ?? B8 ?? ?? ?? ?? 8B 44"), 7),
            ];

            let addr = FNAME_POOL
                .iter()
                .find_map(|(sig, offset)| Some(sig.scan(process, module_range)? + *offset))?;
            addr + 0x4 + process.read::<i32>(addr).ok()?
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

    /// Tries attaching to a UE game. The UE version needs to be correct for this
    /// function to work.
    pub async fn wait_attach(
        process: &Process,
        version: Version,
        main_module_address: Address,
    ) -> Self {
        retry(|| Self::attach(process, version, main_module_address)).await
    }

    /// Returns the memory pointer to GWorld
    pub const fn g_world(&self) -> Address {
        self.g_world
    }

    /// Returns the memory pointer to GEngine
    pub const fn g_engine(&self) -> Address {
        self.g_engine
    }

    /// Returns the current instance of GWorld
    pub fn get_g_world_uobject(&self, process: &Process) -> Option<UObject> {
        match process.read_pointer(self.g_world, self.pointer_size) {
            Ok(Address::NULL) | Err(_) => None,
            Ok(val) => Some(UObject { object: val }),
        }
    }

    /// Returns the current instance of GEngine
    pub fn get_g_engine_uobject(&self, process: &Process) -> Option<UObject> {
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

/// An `UObject` is the base class of every Unreal Engine object,
/// from which every other class in the UE engine inherits from.
///
/// This struct represents a currently running instance of any UE class,
/// from which it's possible to perform introspection in order to return
/// various information, such as the class' `FName`, property names, offsets, etc.
///
// Docs:
// - https://docs.unrealengine.com/4.27/en-US/API/Runtime/CoreUObject/UObject/UObject/
// - https://gist.github.com/apple1417/b23f91f7a9e3b834d6d052d35a0010ff#object-structure
#[derive(Copy, Clone)]
pub struct UObject {
    object: Address,
}

impl UObject {
    /// Reads the `FName` of the current `UObject`
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

    /// Returns the underlying class definition for the current `UObject`
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

/// An UClass / UStruct is the object class relative to a specific UObject.
/// It essentially represents the class definition for any given UObject,
/// containing information about its properties, parent and children classes,
/// and much more.
///
/// It's always referred by an UObject and it's used for recover data about
/// its properties and offsets.
///
// Source: https://github.com/bl-sdk/unrealsdk/blob/master/src/unrealsdk/unreal/classes/ustruct.h
#[derive(Copy, Clone)]
struct UClass {
    class: Address,
}

impl UClass {
    fn properties<'a>(
        &'a self,
        process: &'a Process,
        module: &'a Module,
    ) -> impl FusedIterator<Item = UProperty> + 'a {
        // Logic: properties are contained in a linked list that can be accessed directly
        // through the `property_link` field, from the most derived to the least derived class.
        // Source: https://gist.github.com/apple1417/b23f91f7a9e3b834d6d052d35a0010ff#object-structure
        //
        // However, if you are in a class with no additional fields other than the ones it inherits from,
        // `property_link` results in a null pointer. In this case, we access the parent class
        // through the `super_field` offset.
        let mut current_property = {
            let mut val = None;
            let mut current_class = *self;

            while val.is_none() {
                match process.read_pointer(
                    current_class.class + module.offsets.uclass_property_link,
                    module.pointer_size,
                ) {
                    Ok(Address::NULL) => match process.read_pointer(
                        current_class.class + module.offsets.uclass_super_field,
                        module.pointer_size,
                    ) {
                        Ok(Address::NULL) | Err(_) => break,
                        Ok(super_field) => {
                            current_class = UClass { class: super_field };
                        }
                    },
                    Ok(current_property_address) => {
                        val = Some(UProperty {
                            property: current_property_address,
                        });
                    }
                    _ => break,
                }
            }

            val
        };

        iter::from_fn(move || match current_property {
            Some(prop) => match process.read_pointer(
                prop.property + module.offsets.uproperty_property_link_next,
                module.pointer_size,
            ) {
                Ok(val) => {
                    current_property = match val {
                        Address::NULL => None,
                        _ => Some(UProperty { property: val }),
                    };
                    Some(prop)
                }
                _ => None,
            },
            _ => None,
        })
        .fuse()
    }

    /// Returns the offset for the specified named property.
    /// Returns `None` on case of failure.
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

/// Definition for a property used in a certain UClass.
///
/// Used mostly just to recover field names and offsets.
// Source: https://github.com/bl-sdk/unrealsdk/blob/master/src/unrealsdk/unreal/classes/uproperty.h
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

        // If we already resolved some offsets, we need to traverse them again starting from the base address
        // (usually GWorld of GEngine) in order to recalculate the address of the farthest UObject we can reach.
        // If no offsets have been resolved yet, we just need to read the base address instead.
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

    /// Dereferences the pointer path, returning the memory address at the end of the path
    pub fn deref_offsets(&self, process: &Process, module: &Module) -> Result<Address, Error> {
        self.find_offsets(process, module)?;
        let cache = self.cache.borrow();
        let (&last, path) = cache.offsets[..self.depth].split_last().ok_or(Error {})?;
        let mut address = process.read_pointer(self.base_address, module.pointer_size)?;
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
                Version::V5_3 | Version::V5_4 => &Self {
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
    V5_4,
}
