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
mod pointer;
pub use pointer::UnityPointer;
mod offsets;
pub use offsets::{
    AssemblyOffsets, ClassOffsets, FieldInfoOffsets, GenericOffsets, ImageOffsets, Profile,
    TypeOffsets, TypeStart,
};
#[cfg(all(test, not(target_family = "wasm")))]
mod collections_tests;
pub mod profiles;
#[cfg(all(test, not(target_family = "wasm")))]
mod readers_tests;
#[cfg(all(test, not(target_family = "wasm")))]
mod walk_tests;

use super::{managed, DictionaryOffsets, HashSetOffsets, ListOffsets, ManagedString};

/// Represents access to a Unity game that is using the IL2CPP backend.
pub struct Module {
    assemblies: Address,
    type_info_definition_table: Address,
    profile: Profile,
    pointer_size: PointerSize,
}

impl Module {
    /// Tries attaching to a Unity game that is using the IL2CPP backend. The
    /// game gets the offsets of the measured build nearest to its Unity
    /// version, its own build when someone measured that version.
    pub fn attach_auto_detect(process: &Process) -> Option<Self> {
        let il2cpp_module = Self::find_runtime_module(process)?;
        let pointer_size = pe::MachineType::read(process, il2cpp_module.0)?.pointer_size()?;
        let unity = Self::unity_version(process)?;
        let build = builds::nearest(unity, pointer_size)?;

        let module = Self::attach_with(process, il2cpp_module, build.profile)?;
        print_limited::<128>(&format_args!(
            "il2cpp: unity {}.{}.{}.{} takes the build measured on {}.{}.{}.{}",
            unity.0,
            unity.1,
            unity.2,
            unity.3,
            build.unity.0,
            build.unity.1,
            build.unity.2,
            build.unity.3,
        ));
        Some(module)
    }

    /// Tries attaching to a Unity game that is using the IL2CPP backend with
    /// the provided measured [`Profile`]. The profile's pointer width must
    /// match the target process. Use a built-in [`profiles`] constant for a
    /// known player, or construct a custom profile for another measured
    /// layout. If the target is not known in advance, use
    /// [`attach_auto_detect`](Self::attach_auto_detect) instead.
    pub fn attach(process: &Process, profile: Profile) -> Option<Self> {
        let il2cpp_module = Self::find_runtime_module(process)?;
        let pointer_size = pe::MachineType::read(process, il2cpp_module.0)?.pointer_size()?;
        (pointer_size == profile.pointer_size).then_some(())?;

        Self::attach_with(process, il2cpp_module, profile)
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

    fn attach_with(
        process: &Process,
        il2cpp_module: (Address, u64),
        profile: Profile,
    ) -> Option<Self> {
        let pointer_size = profile.pointer_size;
        let (assemblies, type_info_definition_table) = match pointer_size {
            PointerSize::Bit64 => Self::globals_x64(process, il2cpp_module)?,
            PointerSize::Bit32 => Self::globals_x86(process, il2cpp_module)?,
            _ => return None,
        };

        Some(Self {
            assemblies,
            type_info_definition_table,
            profile,
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

    /// Retrieves the [pointer size](PointerSize) of the target process. This
    /// determines whether managed reference keys and values should be read as
    /// [`Address32`](crate::Address32) or [`Address64`](crate::Address64).
    pub fn get_pointer_size(&self) -> PointerSize {
        self.pointer_size
    }

    fn walk(&self) -> managed::Walk {
        let (metadata_handle, handle_is_inline) = match self.profile.image.type_start {
            TypeStart::Inline(at) => (at, true),
            TypeStart::Handle(at) => (at, false),
        };

        managed::Walk {
            runtime: managed::Runtime::Il2Cpp(managed::Il2CppRuntime {
                assemblies: self.assemblies,
                type_info_definition_table: self.type_info_definition_table,
                type_count: self.profile.image.type_count.into(),
                metadata_handle: metadata_handle.into(),
                handle_is_inline,
                field_count: self.profile.class.field_count,
                static_fields: self.profile.class.static_fields.into(),
                cached_class: self.profile.generic.cached_class,
                type_data: self.profile.type_.data,
                type_kind: self.profile.type_.kind,
            }),
            offsets: managed::WalkOffsets {
                assembly: managed::AssemblyOffsets {
                    name_in_image: self.profile.image.assembly_name.map(u16::from),
                    name_in_assembly: self.profile.assembly.name.map(u16::from),
                    image: self.profile.assembly.image.into(),
                },
                class: managed::ClassOffsets {
                    name: self.profile.class.name.into(),
                    namespace: self.profile.class.namespace.into(),
                    parent: self.profile.class.parent.into(),
                    declaring: self.profile.class.declaring_type,
                    instance_size: self.profile.class.instance_size,
                    fields: self.profile.class.fields.into(),
                },
                field: managed::FieldOffsets {
                    name: self.profile.field.name.into(),
                    type_: self.profile.field.type_,
                    offset: self.profile.field.offset.into(),
                    stride: self.profile.field.size.into(),
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
    /// reference. A string already held by its object address reads with
    /// [`read_string_object`](Self::read_string_object).
    pub fn read_string<const N: usize>(
        &self,
        process: &Process,
        at: Address,
    ) -> Result<ManagedString<N>, Error> {
        managed::read_string(process, self.pointer_size, at)
    }

    /// Reads a managed string at its object address rather than through a
    /// reference: the form for an element
    /// [`read_reference_array`](Self::read_reference_array) hands back. A
    /// null address fails, so a null element refuses at the element with
    /// its index in hand. The returned [`ManagedString`] holds exactly the
    /// string's count of UTF-16 units, a nul character among them like any
    /// other. `N` bounds how many units the string holds, and a string
    /// claiming more than that fails rather than truncates, as does a
    /// negative count.
    pub fn read_string_object<const N: usize>(
        &self,
        process: &Process,
        object: Address,
    ) -> Result<ManagedString<N>, Error> {
        managed::read_string_object(process, self.pointer_size, object)
    }

    /// Reads a managed array of value elements through the reference stored
    /// at the given address. The array carries its own length, so no count
    /// is passed; `N` bounds how many elements the returned [`ArrayVec`]
    /// holds, and an array claiming more than that fails rather than
    /// truncates, as does a null reference. The element type is the caller's
    /// claim and has to match the target's own element layout: a managed
    /// `char` is a `u16` here, a `bool` a single byte, and Rust's `char` and
    /// `usize` never match. Reference elements read with
    /// [`read_reference_array`](Self::read_reference_array).
    pub fn read_array<T: CheckedBitPattern, const N: usize>(
        &self,
        process: &Process,
        at: Address,
    ) -> Result<ArrayVec<T, N>, Error> {
        managed::read_array(process, self.pointer_size, at)
    }

    /// Reads a managed array of reference elements through the reference
    /// stored at the given address, handing back the elements' object
    /// addresses read at the target's own pointer width. A null array
    /// reference fails; null elements are data and come back as null
    /// addresses at their positions, since positions carry meaning and
    /// filtering is the caller's choice. `N` bounds how many elements the
    /// returned vector holds, and an array claiming more than that fails
    /// rather than truncates. A `string` element reads with
    /// [`read_string_object`](Self::read_string_object); any other object's
    /// fields read at its address directly.
    pub fn read_reference_array<const N: usize>(
        &self,
        process: &Process,
        at: Address,
    ) -> Result<ArrayVec<Address, N>, Error> {
        managed::read_reference_array(process, self.pointer_size, at)
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
    /// counts, and how one entry lays out. The address must store a managed
    /// reference to a `System.Collections.Generic.Dictionary`; subclasses
    /// are supported by finding that class in the object's hierarchy.
    ///
    /// The answer is a small `Copy` value worth storing, like a field offset:
    /// resolution walks class metadata, while reading it later costs only a
    /// handful of process reads. The entry layout depends on the dictionary's
    /// concrete key and value types, so the result must only be reused for the
    /// same dictionary type.
    ///
    /// This returns `None` for objects that are not dictionaries, targets
    /// still initializing their metadata, and runtime profiles that lack the
    /// additional layout metadata required for dictionary resolution.
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

    /// Resolves where a `HashSet` keeps its backing slots, live count, and
    /// high-water mark, and how one slot lays out. The address must store a
    /// managed reference to a `System.Collections.Generic.HashSet`;
    /// subclasses are supported by finding that class in the object's
    /// hierarchy.
    ///
    /// The answer is a small `Copy` value worth storing, like a field offset:
    /// resolution walks class metadata, while reading it later costs only a
    /// handful of process reads. The slot layout depends on the hash set's
    /// concrete value type, so the result must only be reused for the same
    /// hash set type.
    ///
    /// This returns `None` for objects that are not hash sets, targets still
    /// initializing their metadata, and runtime profiles that lack the
    /// additional layout metadata required for hash set resolution.
    pub fn get_hash_set_offsets(&self, process: &Process, at: Address) -> Option<HashSetOffsets> {
        let object = process
            .read_pointer(at, self.pointer_size)
            .ok()
            .filter(|address| !address.is_null())?;

        self.walk().hash_set_offsets(process, object)
    }

    /// Reads a managed `HashSet`'s live values through the reference stored
    /// at the given address, with the offsets
    /// [`get_hash_set_offsets`](Self::get_hash_set_offsets) resolved. The
    /// walk runs to the high-water mark rather than the live count, since
    /// freed slots sit inside it; `N` bounds the live values, and the live
    /// tally has to balance against the count exactly or the read fails. The
    /// value type is the caller's claim, as with
    /// [`read_array`](Self::read_array). Managed references are read as
    /// [`Address32`](crate::Address32) or [`Address64`](crate::Address64),
    /// according to [`get_pointer_size`](Self::get_pointer_size).
    pub fn read_hash_set<T: CheckedBitPattern, const N: usize>(
        &self,
        process: &Process,
        offsets: HashSetOffsets,
        at: Address,
    ) -> Result<ArrayVec<T, N>, Error> {
        managed::read_hash_set(process, self.pointer_size, offsets, at)
    }

    /// Reads a managed `Dictionary`'s live pairs through the reference
    /// stored at the given address, with the offsets
    /// [`get_dictionary_offsets`](Self::get_dictionary_offsets) resolved.
    /// `N` bounds the live pairs, never the counted entries or the backing
    /// capacity; freed entries are skipped by their marks, and a live tally
    /// that cannot balance against the counts fails rather than answering
    /// wrong pairs. The key and value types are the caller's claims, as
    /// with [`read_array`](Self::read_array), refused where a claim
    /// outgrows the room its member has inside one entry. Managed references
    /// are read as [`Address32`](crate::Address32) or
    /// [`Address64`](crate::Address64), according to
    /// [`get_pointer_size`](Self::get_pointer_size).
    pub fn read_dictionary<K: CheckedBitPattern, V: CheckedBitPattern, const N: usize>(
        &self,
        process: &Process,
        offsets: DictionaryOffsets,
        at: Address,
    ) -> Result<ArrayVec<(K, V), N>, Error> {
        managed::read_dictionary(process, self.pointer_size, offsets, at)
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

    /// Reads a managed `List` of reference elements through the reference
    /// stored at the given address, with the offsets
    /// [`get_list_offsets`](Self::get_list_offsets) resolved, handing back
    /// the elements' object addresses read at the target's own pointer
    /// width. The list's live count is read, never its backing capacity;
    /// `N` bounds the count, and a count past the buffer or past the
    /// backing array's own length fails rather than truncates, as does a
    /// null list reference. Null elements are data and come back as null
    /// addresses at their positions.
    pub fn read_reference_list<const N: usize>(
        &self,
        process: &Process,
        offsets: ListOffsets,
        at: Address,
    ) -> Result<ArrayVec<Address, N>, Error> {
        managed::read_reference_list(process, self.pointer_size, offsets, at)
    }

    /// Attaches to a Unity game that is using the IL2CPP backend. The game
    /// gets the offsets of the measured build nearest to its Unity version.
    ///
    /// This is the `await`able version of the
    /// [`attach_auto_detect`](Self::attach_auto_detect) function, yielding back
    /// to the runtime between each try.
    pub async fn wait_attach_auto_detect(process: &Process) -> Module {
        retry(|| Self::attach_auto_detect(process)).await
    }

    /// Attaches to a Unity game that is using the IL2CPP backend with the
    /// provided measured [`Profile`]. The profile's pointer width must match
    /// the target process.
    ///
    /// This is the `await`able version of [`attach`](Self::attach), yielding
    /// back to the runtime between each try.
    pub async fn wait_attach(process: &Process, profile: Profile) -> Module {
        retry(|| Self::attach(process, profile)).await
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
    /// counts, and how one entry lays out from the dictionary's class
    /// hierarchy.
    ///
    /// This is the `await`able version of the
    /// [`get_dictionary_offsets`](Self::get_dictionary_offsets) function,
    /// yielding back to the runtime between each try. This waits indefinitely
    /// if the active runtime profile lacks the layout metadata needed for
    /// dictionary resolution.
    pub async fn wait_get_dictionary_offsets(
        &self,
        process: &Process,
        at: Address,
    ) -> DictionaryOffsets {
        retry(|| self.get_dictionary_offsets(process, at)).await
    }

    /// Resolves where a `HashSet` keeps its backing slots, live count, and
    /// high-water mark, and how one slot lays out from the hash set's class
    /// hierarchy.
    ///
    /// This is the `await`able version of the
    /// [`get_hash_set_offsets`](Self::get_hash_set_offsets) function,
    /// yielding back to the runtime between each try. This waits indefinitely
    /// if the active runtime profile lacks the layout metadata needed for
    /// hash set resolution.
    pub async fn wait_get_hash_set_offsets(
        &self,
        process: &Process,
        at: Address,
    ) -> HashSetOffsets {
        retry(|| self.get_hash_set_offsets(process, at)).await
    }
}
