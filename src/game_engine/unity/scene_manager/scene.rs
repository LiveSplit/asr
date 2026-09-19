use super::{offsets::PathShape, SceneManager, CSTR};
use crate::{string::ArrayCString, Address, Error, PointerSize, Process};

/// A scene loaded in the attached game.
pub struct Scene {
    pub(super) address: Address,
}

impl Scene {
    /// Returns the address of the scene in the attached game.
    pub const fn address(&self) -> Address {
        self.address
    }

    /// Returns [`true`] if the address of the scene still points to valid
    /// memory.
    pub fn is_valid(&self, process: &Process) -> bool {
        process.read::<u8>(self.address).is_ok()
    }

    /// Returns the build index of the scene. This index is unique to each
    /// scene in the game.
    pub fn index(&self, process: &Process, scene_manager: &SceneManager) -> Result<i32, Error> {
        process.read(self.address + scene_manager.profile.scene.build_index)
    }

    /// Returns the full asset path of the scene.
    ///
    /// Usually looks like "`Assets/some/path/scene.unity`".
    pub fn path<const N: usize>(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
    ) -> Result<ArrayCString<N>, Error> {
        let at = self.address + scene_manager.profile.scene.path;
        let pointer_size = scene_manager.pointer_size;
        let shape = scene_manager.profile.path;

        if shape == PathShape::Pointer {
            return process
                .read_pointer(at, pointer_size)
                .and_then(|addr| process.read(addr));
        }

        // The field is a union. A path behind a pointer is checked first,
        // because the bytes past an inline path's NUL are unrelated data,
        // and unrelated data must not read as a path.
        let field: [u8; 32] = process.read(at)?;
        if let Some(path) = Self::spilled_path(process, &field, pointer_size) {
            return Ok(path);
        }
        let len = Self::inline_len(&field, shape).ok_or(Error {})?;
        let mut path = ArrayCString::<N>::new();
        let len = len.min(N);
        bytemuck::bytes_of_mut(&mut path)[..len].copy_from_slice(&field[..len]);
        Ok(path)
    }

    /// Reads the path behind the pointer at the start of the field, when the
    /// length after the pointer is believable and the text at the pointer
    /// has that length.
    fn spilled_path<const N: usize>(
        process: &Process,
        field: &[u8; 32],
        pointer_size: PointerSize,
    ) -> Option<ArrayCString<N>> {
        const LONGEST: u64 = 4096;

        let word = |at: usize| -> u64 {
            match pointer_size {
                PointerSize::Bit64 => u64::from_le_bytes(field[at..at + 8].try_into().unwrap()),
                _ => u32::from_le_bytes(field[at..at + 4].try_into().unwrap()) as u64,
            }
        };
        let pointer = Address::new(word(0));
        let len = word(pointer_size as usize);
        if pointer.is_null() || len > LONGEST {
            return None;
        }

        let path: ArrayCString<N> = process.read(pointer).ok()?;
        let read = path.as_bytes().len() as u64;
        (read == len || (read == N as u64 && len > N as u64)).then_some(path)
    }

    /// Returns the length of the path kept inline in the field, or [`None`]
    /// when the field holds no inline path.
    fn inline_len(field: &[u8; 32], shape: PathShape) -> Option<usize> {
        let len = match shape {
            PathShape::InlineSpare => 31_usize.checked_sub(field[31] as usize)?,
            _ => field.iter().position(|&b| b == 0)?,
        };
        let terminated = len == 31 || field[len] == 0;
        (len <= 31 && terminated && !field[..len].contains(&0)).then_some(len)
    }

    /// Returns the full path of the scene, as a [String](alloc::string::String).
    #[cfg(feature = "alloc")]
    pub fn path_as_string(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
    ) -> Result<alloc::string::String, Error> {
        let path = self.path::<CSTR>(process, scene_manager)?;
        let str = path.validate_utf8().map_err(|_| Error {})?;

        Ok(str.into())
    }

    /// Returns the name of the scene, as a [String](alloc::string::String).
    #[cfg(feature = "alloc")]
    pub fn name(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
    ) -> Result<alloc::string::String, Error> {
        // The name is also stored in memory, but it's just easier to interpret the path
        let path = self.path_as_string(process, scene_manager)?;
        // if for some reason the path has no /, or doesn't end in a .unity, just safely default
        let cs = path.rsplit_once('/').unwrap_or(("", &path)).1;
        Ok(cs.rsplit_once('.').unwrap_or((cs, "")).0.into())
    }
}
