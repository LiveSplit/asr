//! The auto splitter the harness runs against every fixture player. It
//! reads what the public API can read of the fixture and reports each
//! value under its path in `unity-fixture.json`, such as
//! `scenes.0.roots.0.name` or `classes.FixtureData.fields.hero.level`,
//! then `done`.

use asr::{
    arrayvec::ArrayVec,
    future::{next_tick, retry},
    game_engine::unity::{il2cpp, mono, scene_manager::SceneManager},
    timer, Address, PointerSize, Process,
};

asr::async_main!(stable);

/// How many UTF-16 units or elements a read holds at most. The fixture's
/// strings and collections are far below it.
const CAP: usize = 64;

async fn main() {
    loop {
        let process = retry(|| {
            ["fixture.exe", "fixture"]
                .iter()
                .find_map(|name| Process::attach(name))
        })
        .await;
        asr::print_message("attached to the player");
        process
            .until_closes(async {
                report(&process).await;
                loop {
                    next_tick().await;
                }
            })
            .await;
    }
}

fn set(key: &str, value: impl AsRef<str>) {
    timer::set_variable(key, value.as_ref());
}

/// The scripting backend a player runs, told by the runtime library it
/// loads. The library comes in some time after the process, so the choice
/// waits for one of them.
enum Backend {
    Mono,
    Il2cpp,
}

async fn report(process: &Process) {
    report_scenes(process).await;
    let backend = retry(|| {
        if has_module(
            process,
            &["GameAssembly.dll", "GameAssembly.so", "GameAssembly.dylib"],
        ) {
            Some(Backend::Il2cpp)
        } else if has_module(
            process,
            &[
                "mono.dll",
                "mono-2.0-bdwgc.dll",
                "libmono.so",
                "libmonobdwgc-2.0.so",
                "libmonobdwgc-2.0.dylib",
            ],
        ) {
            Some(Backend::Mono)
        } else {
            None
        }
    })
    .await;
    match backend {
        Backend::Il2cpp => {
            set("runtime", "il2cpp");
            let module = il2cpp::Module::wait_attach_auto_detect(process).await;
            asr::print_message("attached to il2cpp");
            report_classes(process, &Il2cpp(module)).await;
        }
        Backend::Mono => {
            set("runtime", "mono");
            let module = mono::Module::wait_attach_auto_detect(process).await;
            asr::print_message("attached to mono");
            report_classes(process, &Mono(module)).await;
        }
    }
    set("done", "1");
}

fn has_module(process: &Process, names: &[&str]) -> bool {
    names
        .iter()
        .any(|name| process.get_module_address(name).is_ok())
}

/// Reports the scene list, the Boot scene's hierarchy under `Foo`, and
/// the root under DontDestroyOnLoad. The second scene is loaded by the
/// fixture's own `Awake`, so the list is read once it holds both.
async fn report_scenes(process: &Process) {
    let scenes = SceneManager::wait_attach(process).await;
    asr::print_message("attached to the scene manager");

    // A scene still loading sits in the list with no path and a build
    // index of -1, so the list is read whole until both scenes are in it
    // for real.
    let loaded = retry(|| {
        let loaded: Vec<(String, i32)> = scenes
            .scenes(process)
            .map(|scene| {
                let path = scene.path::<128>(process, &scenes).ok()?;
                let path = String::from(path.validate_utf8().ok()?);
                let index = scene.index(process, &scenes).ok()?;
                (!path.is_empty() && index >= 0).then_some((path, index))
            })
            .collect::<Option<_>>()?;
        (loaded.len() == 2).then_some(loaded)
    })
    .await;
    asr::print_message("both scenes are loaded");
    for (index, (path, build_index)) in loaded.iter().enumerate() {
        set(&format!("scenes.{index}.path"), path);
        timer::set_variable_int(&format!("scenes.{index}.buildIndex"), *build_index);
    }

    let root = retry(|| scenes.get_root_game_object(process, "Foo").ok()).await;
    report_transform(process, &scenes, &root, "scenes.0.roots.0");

    let fixture = retry(|| {
        scenes
            .get_game_object_from_dont_destroy_on_load(process, "Fixture")
            .ok()
    })
    .await;
    report_transform(process, &scenes, &fixture, "dontDestroyOnLoad.roots.0");
}

/// Reports a transform's name and, in order, its children under the
/// path's `children` list.
fn report_transform(
    process: &Process,
    scenes: &SceneManager,
    transform: &asr::game_engine::unity::scene_manager::Transform,
    path: &str,
) {
    if let Ok(name) = transform.get_name::<128>(process, scenes) {
        set(&format!("{path}.name"), name.validate_utf8().unwrap_or(""));
    }
    if let Ok(children) = transform.children(process, scenes) {
        for (index, child) in children.enumerate() {
            report_transform(process, scenes, &child, &format!("{path}.children.{index}"));
        }
    }
}

/// The reads the harness needs from a scripting backend, the same on
/// both.
trait Managed {
    type Class;
    fn pointer_size(&self) -> PointerSize;
    fn class(&self, process: &Process, name: &str) -> Option<Self::Class>;
    fn field_offset(&self, process: &Process, class: &Self::Class, field: &str) -> Option<u32>;
    fn static_table(&self, process: &Process, class: &Self::Class) -> Option<Address>;
    fn string(&self, process: &Process, at: Address) -> Option<String>;
    fn ints(&self, process: &Process, at: Address) -> Option<ArrayVec<i32, CAP>>;
    fn int_list(&self, process: &Process, at: Address) -> Option<ArrayVec<i32, CAP>>;
    fn string_list(&self, process: &Process, at: Address) -> Option<Vec<String>>;
    fn string_int_dictionary(&self, process: &Process, at: Address) -> Option<Vec<(String, i32)>>;
}

struct Mono(mono::Module);
struct Il2cpp(il2cpp::Module);

macro_rules! managed {
    ($wrapper:ident, $class:path) => {
        impl Managed for $wrapper {
            type Class = $class;

            fn pointer_size(&self) -> PointerSize {
                self.0.get_pointer_size()
            }

            fn class(&self, process: &Process, name: &str) -> Option<$class> {
                let image = self.0.get_image(process, "Assembly-CSharp")?;
                image.get_class(process, &self.0, name)
            }

            fn field_offset(&self, process: &Process, class: &$class, field: &str) -> Option<u32> {
                class.get_field_offset(process, &self.0, field)
            }

            fn static_table(&self, process: &Process, class: &$class) -> Option<Address> {
                class.get_static_table(process, &self.0)
            }

            fn string(&self, process: &Process, at: Address) -> Option<String> {
                let string = self.0.read_string::<CAP>(process, at).ok()?;
                Some(String::from_utf16_lossy(&string))
            }

            fn ints(&self, process: &Process, at: Address) -> Option<ArrayVec<i32, CAP>> {
                self.0.read_array::<i32, CAP>(process, at).ok()
            }

            fn int_list(&self, process: &Process, at: Address) -> Option<ArrayVec<i32, CAP>> {
                let offsets = self.0.get_list_offsets(process, at)?;
                self.0.read_list::<i32, CAP>(process, offsets, at).ok()
            }

            fn string_list(&self, process: &Process, at: Address) -> Option<Vec<String>> {
                let offsets = self.0.get_list_offsets(process, at)?;
                let objects = self
                    .0
                    .read_reference_list::<CAP>(process, offsets, at)
                    .ok()?;
                objects
                    .iter()
                    .map(|object| {
                        let string = self.0.read_string_object::<CAP>(process, *object).ok()?;
                        Some(String::from_utf16_lossy(&string))
                    })
                    .collect()
            }

            fn string_int_dictionary(
                &self,
                process: &Process,
                at: Address,
            ) -> Option<Vec<(String, i32)>> {
                let offsets = self.0.get_dictionary_offsets(process, at)?;
                let pairs: Vec<(u64, i32)> = match self.pointer_size() {
                    PointerSize::Bit64 => self
                        .0
                        .read_dictionary::<u64, i32, CAP>(process, offsets, at)
                        .ok()?
                        .into_iter()
                        .collect(),
                    _ => self
                        .0
                        .read_dictionary::<u32, i32, CAP>(process, offsets, at)
                        .ok()?
                        .into_iter()
                        .map(|(key, value)| (key as u64, value))
                        .collect(),
                };
                pairs
                    .into_iter()
                    .map(|(key, value)| {
                        let string = self
                            .0
                            .read_string_object::<CAP>(process, Address::new(key))
                            .ok()?;
                        Some((String::from_utf16_lossy(&string), value))
                    })
                    .collect()
            }
        }
    };
}

managed!(Mono, mono::Class);
managed!(Il2cpp, il2cpp::Class);

/// Reports `FixtureData`'s statics and the fields of its instance under
/// `classes.FixtureData`. `Instance` is set in `Awake`, so the reads wait
/// for it.
async fn report_classes(process: &Process, module: &impl Managed) {
    let class = retry(|| module.class(process, "FixtureData")).await;
    asr::print_message("found FixtureData");
    let statics = retry(|| module.static_table(process, &class)).await;
    let field = |name: &str| module.field_offset(process, &class, name);
    let pointer = module.pointer_size();

    let instance = retry(|| {
        let at = statics + field("Instance")?;
        process
            .read_pointer(at, pointer)
            .ok()
            .filter(|address| !address.is_null())
    })
    .await;
    asr::print_message("FixtureData.Instance is set");

    let prefix = "classes.FixtureData";
    if let Some(offset) = field("StaticInt") {
        if let Ok(value) = process.read::<i32>(statics + offset) {
            timer::set_variable_int(&format!("{prefix}.statics.StaticInt"), value);
        }
    }
    if let Some(value) =
        field("StaticString").and_then(|offset| module.string(process, statics + offset))
    {
        set(&format!("{prefix}.statics.StaticString"), value);
    }

    let at = |name: &str| field(name).map(|offset| instance + offset);
    if let Some(at) = at("intValue") {
        if let Ok(value) = process.read::<i32>(at) {
            timer::set_variable_int(&format!("{prefix}.fields.intValue"), value);
        }
    }
    if let Some(at) = at("longValue") {
        if let Ok(value) = process.read::<i64>(at) {
            timer::set_variable_int(&format!("{prefix}.fields.longValue"), value);
        }
    }
    if let Some(at) = at("floatValue") {
        if let Ok(value) = process.read::<f32>(at) {
            timer::set_variable_float(&format!("{prefix}.fields.floatValue"), value);
        }
    }
    if let Some(at) = at("doubleValue") {
        if let Ok(value) = process.read::<f64>(at) {
            timer::set_variable_float(&format!("{prefix}.fields.doubleValue"), value);
        }
    }
    if let Some(at) = at("boolValue") {
        if let Ok(value) = process.read::<u8>(at) {
            set(
                &format!("{prefix}.fields.boolValue"),
                if value != 0 { "true" } else { "false" },
            );
        }
    }
    if let Some(value) = at("stringValue").and_then(|at| module.string(process, at)) {
        set(&format!("{prefix}.fields.stringValue"), value);
    }

    // `hero` is a `Hero`, a class of its own, so the field holds a
    // reference and the object's fields sit at Hero's offsets.
    let hero = at("hero").and_then(|at| process.read_pointer(at, pointer).ok());
    if let (Some(hero), Some(hero_class)) = (hero, module.class(process, "Hero")) {
        let hero_field = |name: &str| {
            module
                .field_offset(process, &hero_class, name)
                .map(|offset| hero + offset)
        };
        if let Some(at) = hero_field("id") {
            if let Ok(value) = process.read::<i32>(at) {
                timer::set_variable_int(&format!("{prefix}.fields.hero.id"), value);
            }
        }
        if let Some(at) = hero_field("level") {
            if let Ok(value) = process.read::<i32>(at) {
                timer::set_variable_int(&format!("{prefix}.fields.hero.level"), value);
            }
        }
        if let Some(value) = hero_field("title").and_then(|at| module.string(process, at)) {
            set(&format!("{prefix}.fields.hero.title"), value);
        }
    }

    if let Some(values) = at("intArray").and_then(|at| module.ints(process, at)) {
        for (index, value) in values.iter().enumerate() {
            timer::set_variable_int(&format!("{prefix}.fields.intArray.{index}"), *value);
        }
    }
    if let Some(values) = at("intList").and_then(|at| module.int_list(process, at)) {
        for (index, value) in values.iter().enumerate() {
            timer::set_variable_int(&format!("{prefix}.fields.intList.{index}"), *value);
        }
    }
    if let Some(values) = at("stringList").and_then(|at| module.string_list(process, at)) {
        for (index, value) in values.iter().enumerate() {
            set(&format!("{prefix}.fields.stringList.{index}"), value);
        }
    }
    if let Some(pairs) = at("dict").and_then(|at| module.string_int_dictionary(process, at)) {
        for (key, value) in pairs {
            timer::set_variable_int(&format!("{prefix}.fields.dict.{key}"), value);
        }
    }
}
