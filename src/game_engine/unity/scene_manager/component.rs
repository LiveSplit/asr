use crate::game_engine::unity::mono;
use crate::game_engine::unity::scene_manager::SceneManager;
use crate::{Address, Error, Process};

/// Representing a Unity Component, usually attached to a GameObject
#[derive(Debug)]
pub struct Component {
    /// the base address of the component
    pub address: Address,
}

impl Component {
    /** Retrieves the MonoObject this Component holds */
    pub fn get_mono_object(
        &self,
        process: &Process,
        scene_manager: &SceneManager,
        module: &mono::Module,
    ) -> Result<mono::Object, Error> {
        // FIXME: I'm using mono version as a proxy, this has not been well tested and may require changes
        match module.get_version() {
            mono::Version::V1 | mono::Version::V1Cattrs => {
                // FIXME: Understand this better in the mono source code
                //   It's probably similar to the V3 logic
                //   e.g. not sure if "scripting object handle" is correct
                process
                    .read_pointer(
                        self.address + scene_manager.offsets.scripting_object_handle,
                        scene_manager.pointer_size,
                    )
                    .ok()
                    .filter(|val| !val.is_null())
                    .map(|address| mono::Object { address })
                    .ok_or(Error {})
            }
            // FIXME: Untested on V2
            mono::Version::V2 | mono::Version::V3 => {
                // NOTE: I'm like 80% sure the following is true
                process
                    // class ScriptingGCHandle m_MonoReference
                    // See https://gist.github.com/just-ero/92457b51baf85bd1e5b8c87de8c9835e#file-object-hpp-L18
                    // In Mono this is a MonoObjectHandle, which is a MonoObject**
                    // MonoObject **__raw
                    // See https://github.com/mono/mono/blob/0f53e9e151d92944cacab3e24ac359410c606df6/mono/metadata/handle-decl.h#L72
                    .read_pointer(
                        self.address + scene_manager.offsets.scripting_object_handle,
                        scene_manager.pointer_size,
                    )
                    .ok()
                    .filter(|val| !val.is_null())
                    // MonoObject *__raw
                    .and_then(|addr| process.read_pointer(addr, scene_manager.pointer_size).ok())
                    .filter(|val| !val.is_null())
                    .map(|address| mono::Object { address })
                    .ok_or(Error {})
            }
        }
    }
}
