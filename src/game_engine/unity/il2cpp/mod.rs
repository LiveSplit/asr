//! Support for attaching to Unity games that are using the IL2CPP backend.

use arrayvec::ArrayVec;
use bytemuck::CheckedBitPattern;

use crate::{
    file_format::pe, future::retry, print_limited, signature::Signature, Address, Error,
    PointerSize, Process,
};

mod builds;
mod image;
pub use image::Image;
mod class;
pub use class::Class;
mod version;
pub use version::Version;
mod pointer;
pub use pointer::UnityPointer;
mod offsets;
use offsets::IL2CPPOffsets;
#[cfg(all(test, not(target_family = "wasm")))]
mod collections_tests;
#[cfg(all(test, not(target_family = "wasm")))]
mod readers_tests;
#[cfg(all(test, not(target_family = "wasm")))]
mod walk_tests;

use super::{managed, DictionaryOffsets, ListOffsets, ManagedString};

/// Represents access to a Unity game that is using the IL2CPP backend.
pub struct Module {
    assemblies: Address,
    type_info_definition_table: Address,
    version: Version,
    offsets: &'static IL2CPPOffsets,
    pointer_size: PointerSize,
}

impl Module {
    /// Tries attaching to a Unity game that is using the IL2CPP backend. If
    /// the game's metadata and Unity versions name a measured build, this
    /// function uses that build's offsets. Otherwise this function detects
    /// the [IL2CPP version](Version). If you know the version in advance or
    /// it fails detecting it, use [`attach`](Self::attach) instead.
    pub fn attach_auto_detect(process: &Process) -> Option<Self> {
        let il2cpp_module = Self::find_runtime_module(process)?;
        let pointer_size = pe::MachineType::read(process, il2cpp_module.0)?.pointer_size()?;

        let unity = Self::unity_version(process);
        let metadata = Self::metadata_version(process);

        match (unity, metadata) {
            (Some(unity), Some(metadata)) => {
                if let Some(build) = builds::find(metadata, unity, pointer_size) {
                    let module = Self::attach_with(
                        process,
                        il2cpp_module,
                        pointer_size,
                        build.version,
                        &build.offsets,
                    )?;
                    print_limited::<128>(&format_args!(
                        "known il2cpp build: metadata {metadata}, unity {}.{}.{}.{}",
                        unity.0, unity.1, unity.2, unity.3,
                    ));
                    return Some(module);
                }
                print_limited::<128>(&format_args!(
                    "unknown il2cpp build: metadata {metadata}, unity {}.{}.{}.{}",
                    unity.0, unity.1, unity.2, unity.3,
                ));
            }
            // The game maps its metadata after GameAssembly.dll. A player
            // with a measured build waits for it, since attaching now would
            // take the version table's offsets and keep them.
            (Some(unity), None) if builds::measured(unity, pointer_size) => return None,
            _ => {}
        }

        let version = Version::detect(process)?;
        Self::attach(process, version)
    }

    /// Tries attaching to a Unity game that is using the IL2CPP backend with
    /// the [IL2CPP version](Version) provided. The version needs to be
    /// correct for this function to work. If you don't know the version in
    /// advance, use [`attach_auto_detect`](Self::attach_auto_detect) instead.
    pub fn attach(process: &Process, version: Version) -> Option<Self> {
        let il2cpp_module = Self::find_runtime_module(process)?;
        let pointer_size = pe::MachineType::read(process, il2cpp_module.0)?.pointer_size()?;
        let offsets = IL2CPPOffsets::new(version, pointer_size)?;

        Self::attach_with(process, il2cpp_module, pointer_size, version, offsets)
    }

    fn find_runtime_module(process: &Process) -> Option<(Address, u64)> {
        let address = process.get_module_address("GameAssembly.dll").ok()?;
        let size = pe::read_size_of_image(process, address)? as u64;
        Some((address, size))
    }

    /// Reads the Unity version stamped on the player, all four parts of
    /// `UnityPlayer.dll`'s file version.
    fn unity_version(process: &Process) -> Option<(u16, u16, u16, u16)> {
        let unity_player = process.get_module_address("UnityPlayer.dll").ok()?;
        let file_version = pe::FileVersion::read(process, unity_player)?;
        Some((
            file_version.major_version,
            file_version.minor_version,
            file_version.build_part,
            file_version.private_part,
        ))
    }

    /// Reads the version of the game's metadata. The mapped
    /// `global-metadata.dat` starts with a sanity value, then the version.
    fn metadata_version(process: &Process) -> Option<u32> {
        process.memory_ranges().find_map(|range| {
            let [sanity, version] = process.read::<[u32; 2]>(range.address().ok()?).ok()?;
            // Versions are small numbers. Unity 6 renumbered them and reaches
            // the low hundreds.
            (sanity == 0xFAB1_1BAF && (16..=999).contains(&version)).then_some(version)
        })
    }

    fn attach_with(
        process: &Process,
        il2cpp_module: (Address, u64),
        pointer_size: PointerSize,
        version: Version,
        offsets: &'static IL2CPPOffsets,
    ) -> Option<Self> {
        let (assemblies, type_info_definition_table) = match pointer_size {
            PointerSize::Bit64 => Self::globals_x64(process, il2cpp_module)?,
            PointerSize::Bit32 => Self::globals_x86(process, il2cpp_module)?,
            _ => return None,
        };

        Some(Self {
            assemblies,
            type_info_definition_table,
            version,
            offsets,
            pointer_size,
        })
    }

    /// Finds the assemblies vector and the type-info table in x64 code. Each
    /// is given as a displacement from the next instruction.
    fn globals_x64(process: &Process, il2cpp_module: (Address, u64)) -> Option<(Address, Address)> {
        let displaced = |addr: Address| Some(addr + 0x4 + process.read::<i32>(addr).ok()?);

        // jne; mov rbx, [begin]; cmp rbx, [end]. The end of a vector sits one
        // pointer past its begin.
        const ASSEMBLIES: Signature<16> =
            Signature::new("75 ?? 48 8B 1D ?? ?? ?? ?? 48 3B 1D ?? ?? ?? ??");
        let assemblies = ASSEMBLIES
            .scan_iter(process, il2cpp_module)
            .find_map(|addr| {
                let begin = displaced(addr + 5)?;
                (displaced(addr + 12)? == begin + 8u64).then_some(begin)
            })?;

        let s_metadata = Self::metadata_name(process, il2cpp_module)?;

        // lea rcx, [name]
        const LEA: Signature<7> = Signature::new("48 8D 0D ?? ?? ?? ??");
        let lea: Address = LEA
            .scan_iter(process, il2cpp_module)
            .map(|addr| addr + 3)
            .find(|&addr| displaced(addr) == Some(s_metadata))?;

        // shr rcx, imm8, then mov [table], rax
        const SHR: Signature<3> = Signature::new("48 C1 E9");
        let shr: Address = SHR
            .scan_process_range(process, Self::within(il2cpp_module, lea, 0x200))
            .map(|addr| addr + 3)?;

        const RAX: Signature<7> = Signature::new("48 89 05 ?? ?? ?? ??");
        let table = RAX
            .scan_process_range(process, Self::within(il2cpp_module, shr, 0x100))
            .map(|addr| addr + 3)
            .and_then(displaced)?;

        Self::inside(il2cpp_module, assemblies, table)
    }

    /// Finds the assemblies vector and the type-info table in x86 code. Each
    /// is given as an absolute address.
    fn globals_x86(process: &Process, il2cpp_module: (Address, u64)) -> Option<(Address, Address)> {
        let absolute = |addr: Address| Some(Address::new(process.read::<u32>(addr).ok()? as u64));

        // jne; mov esi, [begin]; sub edi, ecx; cmp esi, [end]. The end of a
        // vector sits one pointer past its begin.
        const ASSEMBLIES: Signature<16> =
            Signature::new("75 ?? 8B 35 ?? ?? ?? ?? 2B F9 3B 35 ?? ?? ?? ??");
        let assemblies = ASSEMBLIES
            .scan_iter(process, il2cpp_module)
            .find_map(|addr| {
                let begin = absolute(addr + 4)?;
                (absolute(addr + 12)? == begin + 4u64).then_some(begin)
            })?;

        let s_metadata = Self::metadata_name(process, il2cpp_module)?;

        // push offset name; call
        const PUSH: Signature<6> = Signature::new("68 ?? ?? ?? ?? E8");
        let push: Address = PUSH
            .scan_iter(process, il2cpp_module)
            .map(|addr| addr + 1)
            .find(|&addr| absolute(addr) == Some(s_metadata))?;

        // The table is the first store after the name. Three shapes store
        // it. Through Unity 6000.2 the count is a byte size, so a shift
        // divides it first, and some of those builds reload ecx after the
        // call. From 6000.3 the count comes straight from the header, at
        // 0xF4.
        const DIVIDED: Signature<14> = Signature::new("C1 EA ?? 52 E8 ?? ?? ?? ?? A3 ?? ?? ?? ??");
        const DIVIDED_RELOAD: Signature<20> =
            Signature::new("C1 EA ?? 52 E8 ?? ?? ?? ?? 8B 0D ?? ?? ?? ?? A3 ?? ?? ?? ??");
        const PUSHED: Signature<16> =
            Signature::new("FF B0 F4 00 00 00 E8 ?? ?? ?? ?? A3 ?? ?? ?? ??");
        let window = Self::within(il2cpp_module, push, 0x400);
        let store = [
            DIVIDED
                .scan_process_range(process, window)
                .map(|addr| addr + 10),
            DIVIDED_RELOAD
                .scan_process_range(process, window)
                .map(|addr| addr + 16),
            PUSHED
                .scan_process_range(process, window)
                .map(|addr| addr + 12),
        ]
        .into_iter()
        .flatten()
        .min()?;
        let table = absolute(store)?;

        Self::inside(il2cpp_module, assemblies, table)
    }

    /// Finds the string `global-metadata.dat` in the module.
    fn metadata_name(process: &Process, il2cpp_module: (Address, u64)) -> Option<Address> {
        const GLOBAL_METADATA: Signature<20> =
            Signature::new("67 6C 6F 62 61 6C 2D 6D 65 74 61 64 61 74 61 2E 64 61 74 00");
        GLOBAL_METADATA.scan_process_range(process, il2cpp_module)
    }

    /// The range from `start` for up to `len` bytes, cut at the module's end.
    fn within(module: (Address, u64), start: Address, len: u64) -> (Address, u64) {
        let end = module.0.value().saturating_add(module.1);
        (start, len.min(end.saturating_sub(start.value())))
    }

    /// The two globals, once both lie inside the module.
    fn inside(
        module: (Address, u64),
        assemblies: Address,
        table: Address,
    ) -> Option<(Address, Address)> {
        let end = module.0.value().saturating_add(module.1);
        let holds = |addr: Address| (module.0.value()..end).contains(&addr.value());
        (holds(assemblies) && holds(table)).then_some((assemblies, table))
    }

    fn walk(&self) -> managed::Walk {
        managed::Walk {
            runtime: managed::Runtime::Il2Cpp(managed::Il2CppRuntime {
                assemblies: self.assemblies,
                type_info_definition_table: self.type_info_definition_table,
                type_count: self.offsets.image.type_count.into(),
                metadata_handle: self.offsets.image.metadata_handle.into(),
                handle_is_inline: matches!(self.version, Version::Base | Version::V2019),
                field_count: self.offsets.class.field_count,
                static_fields: self.offsets.class.static_fields.into(),
                cached_class: self.offsets.generic.cached_class,
                type_data: self.offsets.type_words.data,
                type_kind: self.offsets.type_words.kind,
            }),
            offsets: managed::WalkOffsets {
                assembly: managed::AssemblyOffsets {
                    name_in_image: self.offsets.image.assembly_name.map(u16::from),
                    name_in_assembly: self.offsets.assembly.aname.map(u16::from),
                    image: self.offsets.assembly.image.into(),
                },
                class: managed::ClassOffsets {
                    name: self.offsets.class.name.into(),
                    namespace: self.offsets.class.namespace.into(),
                    parent: self.offsets.class.parent.into(),
                    declaring: self.offsets.class.declaring_type,
                    instance_size: self.offsets.class.instance_size,
                    fields: self.offsets.class.fields.into(),
                },
                field: managed::FieldOffsets {
                    name: self.offsets.field.name.into(),
                    type_: self.offsets.field.type_,
                    offset: self.offsets.field.offset.into(),
                    stride: self.offsets.field.struct_size.into(),
                },
            },
            stop: managed::ClimbStop::UNITY,
            pointer_size: self.pointer_size,
        }
    }

    /// Looks for the specified binary [image](Image) inside the target process.
    /// An [image](Image) is a .NET DLL that is loaded
    /// by the game. The `Assembly-CSharp` [image](Image) is the main game
    /// assembly, and contains all the game logic. The
    /// [`get_default_image`](Self::get_default_image) function is a shorthand
    /// for this function that accesses the `Assembly-CSharp` [image](Image).
    pub fn get_image(&self, process: &Process, assembly_name: &str) -> Option<Image> {
        self.walk()
            .find_image(process, assembly_name)
            .map(|image| Image {
                image: image.address,
            })
    }

    /// Looks for the `Assembly-CSharp` binary [image](Image) inside the target
    /// process. An [image](Image) is a .NET DLL that is loaded
    /// by the game. The `Assembly-CSharp` [image](Image) is the main
    /// game assembly, and contains all the game logic. This function is a
    /// shorthand for [`get_image`](Self::get_image) that accesses the
    /// `Assembly-CSharp` [image](Image).
    pub fn get_default_image(&self, process: &Process) -> Option<Image> {
        self.get_image(process, "Assembly-CSharp")
    }

    /// Reads a managed string through the reference stored at the given
    /// address, such as the end of a pointer path or a slot in a static
    /// table. The string carries its own character count, so no length is
    /// passed, and the returned [`ManagedString`] holds exactly that many
    /// UTF-16 units, a nul character among them like any other. `N` bounds
    /// how many units the string holds, and a string claiming more than that
    /// fails rather than truncates, as do a negative count and a null
    /// reference.
    pub fn read_string<const N: usize>(
        &self,
        process: &Process,
        at: Address,
    ) -> Result<ManagedString<N>, Error> {
        managed::read_string(process, self.pointer_size, at)
    }

    /// Reads a managed array of value elements through the reference stored
    /// at the given address. The array carries its own length, so no count
    /// is passed; `N` bounds how many elements the returned [`ArrayVec`]
    /// holds, and an array claiming more than that fails rather than
    /// truncates, as does a null reference. The element type is the caller's
    /// claim and has to match the target's own element layout: a managed
    /// `char` is a `u16` here, a `bool` a single byte, and Rust's `char` and
    /// `usize` never match. Reference elements have no portable claim.
    pub fn read_array<T: CheckedBitPattern, const N: usize>(
        &self,
        process: &Process,
        at: Address,
    ) -> Result<ArrayVec<T, N>, Error> {
        managed::read_array(process, self.pointer_size, at)
    }

    /// Resolves where a `List` keeps its backing array and live count, finding
    /// `System.Collections.Generic.List` in the class hierarchy of the object
    /// at the given address. The answer is a small `Copy` value worth storing,
    /// like a field offset: resolution walks class metadata, where the read
    /// itself is a handful of reads. An object that is not a list misses.
    pub fn get_list_offsets(&self, process: &Process, at: Address) -> Option<ListOffsets> {
        let object = process
            .read_pointer(at, self.pointer_size)
            .ok()
            .filter(|address| !address.is_null())?;

        self.walk().list_offsets(process, object)
    }

    /// Resolves where a `Dictionary` keeps its backing entries and live
    /// counts, and how one entry lays out, off the class the dictionary
    /// object at the given address names as its own. The answer is a small
    /// `Copy` value worth storing, like a field offset: resolution walks
    /// class metadata, where the read itself is a handful of reads. An
    /// object whose class is not this dictionary shape, and a target still
    /// starting up, both miss.
    pub fn get_dictionary_offsets(
        &self,
        process: &Process,
        at: Address,
    ) -> Option<DictionaryOffsets> {
        let object = process
            .read_pointer(at, self.pointer_size)
            .ok()
            .filter(|address| !address.is_null())?;

        self.walk().dictionary_offsets(process, object)
    }

    /// Reads a managed `Dictionary`'s live pairs through the reference
    /// stored at the given address, with the offsets
    /// [`get_dictionary_offsets`](Self::get_dictionary_offsets) resolved.
    /// `N` bounds the live pairs, never the counted entries or the backing
    /// capacity; freed entries are skipped by their marks, and a live tally
    /// that cannot balance against the counts fails rather than answering
    /// wrong pairs. The key and value types are the caller's claims, as
    /// with [`read_array`](Self::read_array), refused where a claim
    /// outgrows the room its member has inside one entry.
    pub fn read_dictionary<K: CheckedBitPattern, V: CheckedBitPattern, const N: usize>(
        &self,
        process: &Process,
        offsets: DictionaryOffsets,
        at: Address,
    ) -> Result<ArrayVec<(K, V), N>, Error> {
        managed::read_dictionary(process, self.pointer_size, offsets, at)
    }

    /// Returns the pointer size the target runs at, which is what a caller
    /// claims reference-width values with.
    pub fn get_pointer_size(&self) -> PointerSize {
        self.pointer_size
    }

    /// Reads a managed `List` of value elements through the reference stored
    /// at the given address, with the offsets
    /// [`get_list_offsets`](Self::get_list_offsets) resolved. The list's
    /// live count is read, never its backing capacity; `N` bounds the
    /// count, and a count past the buffer or past the backing array's own
    /// length fails rather than truncates, as does a null reference. The
    /// element type is the caller's claim, as with
    /// [`read_array`](Self::read_array).
    pub fn read_list<T: CheckedBitPattern, const N: usize>(
        &self,
        process: &Process,
        offsets: ListOffsets,
        at: Address,
    ) -> Result<ArrayVec<T, N>, Error> {
        managed::read_list(process, self.pointer_size, offsets, at)
    }

    /// Attaches to a Unity game that is using the IL2CPP backend. This function
    /// automatically detects the [IL2CPP version](Version). If you know the
    /// version in advance or it fails detecting it, use
    /// [`wait_attach`](Self::wait_attach) instead.
    ///
    /// This is the `await`able version of the
    /// [`attach_auto_detect`](Self::attach_auto_detect) function, yielding back
    /// to the runtime between each try.
    pub async fn wait_attach_auto_detect(process: &Process) -> Module {
        retry(|| Self::attach_auto_detect(process)).await
    }

    /// Attaches to a Unity game that is using the IL2CPP backend with the
    /// [IL2CPP version](Version) provided. The version needs to be correct
    /// for this function to work. If you don't know the version in advance, use
    /// [`wait_attach_auto_detect`](Self::wait_attach_auto_detect) instead.
    ///
    /// This is the `await`able version of the [`attach`](Self::attach)
    /// function, yielding back to the runtime between each try.
    pub async fn wait_attach(process: &Process, version: Version) -> Module {
        retry(|| Self::attach(process, version)).await
    }

    /// Looks for the specified binary [image](Image) inside the target process.
    /// An [image](Image) is a .NET DLL that is loaded
    /// by the game. The `Assembly-CSharp` [image](Image) is the main game
    /// assembly, and contains all the game logic. The
    /// [`wait_get_default_image`](Self::wait_get_default_image) function is a
    /// shorthand for this function that accesses the `Assembly-CSharp`
    /// [image](Image).
    ///
    /// This is the `await`able version of the [`get_image`](Self::get_image)
    /// function, yielding back to the runtime between each try.
    pub async fn wait_get_image(&self, process: &Process, assembly_name: &str) -> Image {
        retry(|| self.get_image(process, assembly_name)).await
    }

    /// Looks for the `Assembly-CSharp` binary [image](Image) inside the target
    /// process. An [image](Image) is a .NET DLL that
    /// is loaded by the game. The `Assembly-CSharp` [image](Image) is the main
    /// game assembly, and contains all the game logic. This function is a
    /// shorthand for [`wait_get_image`](Self::wait_get_image) that accesses the
    /// `Assembly-CSharp` [image](Image).
    ///
    /// This is the `await`able version of the
    /// [`get_default_image`](Self::get_default_image) function, yielding back
    /// to the runtime between each try.
    pub async fn wait_get_default_image(&self, process: &Process) -> Image {
        retry(|| self.get_default_image(process)).await
    }

    /// Resolves where a `List` keeps its backing array and live count from the
    /// class hierarchy of the list object at the given address.
    ///
    /// This is the `await`able version of the
    /// [`get_list_offsets`](Self::get_list_offsets) function, yielding back
    /// to the runtime between each try.
    pub async fn wait_get_list_offsets(&self, process: &Process, at: Address) -> ListOffsets {
        retry(|| self.get_list_offsets(process, at)).await
    }

    /// Resolves where a `Dictionary` keeps its backing entries and live
    /// counts, and how one entry lays out, off the class the dictionary
    /// object at the given address names as its own.
    ///
    /// This is the `await`able version of the
    /// [`get_dictionary_offsets`](Self::get_dictionary_offsets) function,
    /// yielding back to the runtime between each try.
    pub async fn wait_get_dictionary_offsets(
        &self,
        process: &Process,
        at: Address,
    ) -> DictionaryOffsets {
        retry(|| self.get_dictionary_offsets(process, at)).await
    }
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::Module;
    use crate::runtime::mock::with_process;

    #[test]
    fn reads_the_metadata_version_off_the_mapped_file() {
        let mapped = [0xAF_u8, 0x1B, 0xB1, 0xFA, 39, 0, 0, 0];
        // The sanity value with nothing sane behind it must not answer.
        let stray = [0xAF_u8, 0x1B, 0xB1, 0xFA, 0, 0, 0, 0];

        with_process(&[(0x10000, &stray), (0x20000, &mapped)], |process| {
            assert_eq!(Module::metadata_version(process), Some(39));
        });

        with_process(&[(0x10000, &stray)], |process| {
            assert!(Module::metadata_version(process).is_none());
        });
    }
}
