//! Runs the Unity fixture players against an auto splitter built on asr
//! and checks what it reads against the fixtures repo's contract.

use std::{
    collections::BTreeMap,
    fmt, fs,
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use livesplit_auto_splitting::{
    settings, CompiledAutoSplitter, Config, LogLevel, Runtime, Timer, TimerState,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// The commit of the fixtures repo whose manifest and contract are used.
const FIXTURES: &str = "111c92417be1843101d3cd17300a049bd56ec15d";

/// One asset of the fixtures repo: a player for one Unity version and
/// variant, with what the manifest says about the zip.
#[derive(Deserialize)]
struct Entry {
    version: String,
    variant: String,
    asset: String,
    size: u64,
    sha256: String,
}

#[derive(Deserialize)]
struct Manifest {
    fixtures: Vec<Entry>,
}

/// The name of the zip an entry points at.
fn zip_name(entry: &Entry) -> &str {
    entry.asset.rsplit('/').next().unwrap_or(&entry.asset)
}

/// The entries whose players run on a platform, such as `win` or `linux`.
fn on<'a>(platform: &'a str, entries: &'a [Entry]) -> impl Iterator<Item = &'a Entry> {
    entries.iter().filter(move |entry| {
        entry.variant.starts_with(platform) && entry.variant[platform.len()..].starts_with('-')
    })
}

/// The platform this harness runs on, in the words the variants use.
fn host() -> &'static str {
    match std::env::consts::OS {
        "windows" => "win",
        "macos" => "mac",
        os => os,
    }
}

/// Checks a zip against the size and digest the manifest holds for it.
fn verify(bytes: &[u8], entry: &Entry) -> Result<(), String> {
    if bytes.len() as u64 != entry.size {
        return Err(format!(
            "{} is {} bytes, the manifest says {}",
            zip_name(entry),
            bytes.len(),
            entry.size
        ));
    }
    let digest = format!("{:x}", Sha256::digest(bytes));
    if digest != entry.sha256 {
        return Err(format!(
            "{} hashes {digest}, the manifest says {}",
            zip_name(entry),
            entry.sha256
        ));
    }
    Ok(())
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    process::exit(1);
}

/// Downloads one URL whole.
fn fetch(url: &str) -> Vec<u8> {
    let mut response = ureq::get(url)
        .call()
        .unwrap_or_else(|error| fail(&format!("fetching {url}: {error}")));
    response
        .body_mut()
        .with_config()
        .limit(4 << 30)
        .read_to_vec()
        .unwrap_or_else(|error| fail(&format!("reading {url}: {error}")))
}

/// The directory the zips and the unpacked players are kept in, next to
/// the build output unless `ASR_FIXTURES` says otherwise.
fn cache() -> PathBuf {
    match std::env::var_os("ASR_FIXTURES") {
        Some(dir) => PathBuf::from(dir),
        None => Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("fixtures"),
    }
}

/// The directory an entry's player is unpacked into, fetching and checking
/// the zip on the way when the cache has no good copy of it.
fn player(entry: &Entry, cache: &Path) -> PathBuf {
    let name = zip_name(entry);
    let zip = cache.join(name);
    let unpacked = cache.join(name.strip_suffix(".zip").unwrap_or(name));
    if unpacked.is_dir() {
        return unpacked;
    }

    let bytes = match fs::read(&zip)
        .ok()
        .filter(|bytes| verify(bytes, entry).is_ok())
    {
        Some(bytes) => bytes,
        None => {
            println!("fetching {name}");
            let bytes = fetch(&entry.asset);
            verify(&bytes, entry).unwrap_or_else(|error| fail(&error));
            fs::write(&zip, &bytes).expect("writing the zip");
            bytes
        }
    };

    // The player is unpacked beside its final place and moved there whole,
    // so a run that dies halfway leaves no directory that passes as done.
    let part = cache.join(format!("{}.part", zip_name(entry)));
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("opening the zip");
    archive.extract(&part).expect("unpacking the player");
    fs::rename(&part, &unpacked).expect("moving the player into place");
    unpacked
}

/// Every value in the contract under its path: object keys and array
/// indices joined by dots, such as `scenes.0.roots.0.name`.
fn leaves(contract: &serde_json::Value) -> BTreeMap<String, String> {
    fn walk(value: &serde_json::Value, path: &str, into: &mut BTreeMap<String, String>) {
        let child = |key: &str| match path {
            "" => key.to_string(),
            _ => format!("{path}.{key}"),
        };
        match value {
            serde_json::Value::Object(fields) => {
                for (key, value) in fields {
                    walk(value, &child(key), into);
                }
            }
            serde_json::Value::Array(items) => {
                for (index, value) in items.iter().enumerate() {
                    walk(value, &child(&index.to_string()), into);
                }
            }
            serde_json::Value::String(text) => {
                into.insert(path.to_string(), text.clone());
            }
            _ => {
                into.insert(path.to_string(), value.to_string());
            }
        }
    }
    let mut into = BTreeMap::new();
    walk(contract, "", &mut into);
    into
}

/// The paths the auto splitter reads today. A player passes when every
/// one is reported and everything reported matches the contract.
const REQUIRED: &[&str] = &[
    "scenes.0.path",
    "scenes.0.buildIndex",
    "scenes.1.path",
    "scenes.1.buildIndex",
    "scenes.0.roots.0.name",
    "scenes.0.roots.0.children.0.name",
    "scenes.0.roots.0.children.0.children.0.name",
    "scenes.0.roots.0.children.0.children.0.children.0.name",
    "scenes.0.roots.0.children.0.children.0.children.1.name",
    "scenes.0.roots.0.children.0.children.0.children.1.children.0.name",
    "dontDestroyOnLoad.roots.0.name",
    "classes.FixtureData.statics.StaticInt",
    "classes.FixtureData.statics.StaticString",
    "classes.FixtureData.fields.intValue",
    "classes.FixtureData.fields.longValue",
    "classes.FixtureData.fields.floatValue",
    "classes.FixtureData.fields.doubleValue",
    "classes.FixtureData.fields.boolValue",
    "classes.FixtureData.fields.stringValue",
    "classes.FixtureData.fields.hero.id",
    "classes.FixtureData.fields.hero.level",
    "classes.FixtureData.fields.hero.title",
    "classes.FixtureData.fields.intArray.0",
    "classes.FixtureData.fields.intArray.1",
    "classes.FixtureData.fields.intArray.2",
    "classes.FixtureData.fields.intList.0",
    "classes.FixtureData.fields.intList.1",
    "classes.FixtureData.fields.intList.2",
    "classes.FixtureData.fields.stringList.0",
    "classes.FixtureData.fields.stringList.1",
    "classes.FixtureData.fields.stringList.2",
    "classes.FixtureData.fields.dict.one",
    "classes.FixtureData.fields.dict.two",
    "classes.FixtureData.fields.dict.three",
];

/// The variables the auto splitter sets about the run itself, not about
/// the contract.
const ABOUT_THE_RUN: &[&str] = &["runtime", "done"];

/// Compares what the auto splitter reported with the contract. Numbers
/// compare as numbers, so `2.50` matches `2.5`.
fn check(
    reported: &BTreeMap<String, String>,
    contract: &BTreeMap<String, String>,
    required: &[&str],
) -> Vec<String> {
    let same = |a: &str, b: &str| {
        a == b || matches!((a.parse::<f64>(), b.parse::<f64>()), (Ok(a), Ok(b)) if a == b)
    };
    let mut problems = Vec::new();
    for (path, value) in reported {
        if ABOUT_THE_RUN.contains(&path.as_str()) {
            continue;
        }
        match contract.get(path) {
            Some(expected) if same(value, expected) => {}
            Some(expected) => {
                problems.push(format!("{path} is {value}, the contract says {expected}"))
            }
            None => problems.push(format!("{path} is not in the contract")),
        }
    }
    for path in required {
        if !reported.contains_key(*path) {
            problems.push(format!("{path} is missing"));
        }
    }
    problems
}

/// What the auto splitter told the timer: the variables it set and the
/// messages it printed.
#[derive(Default)]
struct Reported {
    variables: BTreeMap<String, String>,
    messages: Vec<String>,
}

/// A timer that only remembers. The auto splitter never starts or splits;
/// it reports through variables.
struct Report(Arc<Mutex<Reported>>);

impl Timer for Report {
    fn state(&self) -> TimerState {
        TimerState::NotRunning
    }
    fn current_split_index(&self) -> Option<usize> {
        None
    }
    fn segment_splitted(&self, _index: usize) -> Option<bool> {
        None
    }
    fn start(&mut self) {}
    fn split(&mut self) {}
    fn skip_split(&mut self) {}
    fn undo_split(&mut self) {}
    fn reset(&mut self) {}
    fn set_game_time(&mut self, _time: time::Duration) {}
    fn pause_game_time(&mut self) {}
    fn resume_game_time(&mut self) {}
    fn set_variable(&mut self, key: &str, value: &str) {
        self.0
            .lock()
            .unwrap()
            .variables
            .insert(key.into(), value.into());
    }
    fn log_auto_splitter(&mut self, message: fmt::Arguments) {
        self.0.lock().unwrap().messages.push(message.to_string());
    }
    fn log_runtime(&mut self, message: fmt::Arguments, _level: LogLevel) {
        self.0.lock().unwrap().messages.push(message.to_string());
    }
}

/// Builds the auto splitter for wasm and reads the module back.
fn splitter() -> Vec<u8> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let status = Command::new(env!("CARGO"))
        .args([
            "build",
            "--release",
            "-p",
            "splitter",
            "--target",
            "wasm32-unknown-unknown",
        ])
        .current_dir(root)
        .status()
        .expect("running cargo");
    if !status.success() {
        fail("the auto splitter did not build");
    }
    fs::read(root.join("target/wasm32-unknown-unknown/release/splitter.wasm"))
        .expect("reading the auto splitter")
}

/// How long a player gets to start and be read before the run counts as
/// stuck.
const PATIENCE: Duration = Duration::from_secs(90);

/// Starts a player headless, runs the auto splitter against it until it
/// says it is done or the patience runs out, and stops the player.
fn run(player: &Path, splitter: &CompiledAutoSplitter, log: &Path) -> Reported {
    let executable = player.join(if cfg!(windows) {
        "fixture.exe"
    } else {
        "fixture"
    });
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .expect("marking the player executable");
    }
    let mut child = Command::new(&executable)
        .args(["-batchmode", "-nographics", "-logFile"])
        .arg(log)
        .current_dir(player)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| fail(&format!("starting {}: {error}", executable.display())));

    let reported = Arc::new(Mutex::new(Reported::default()));
    let auto_splitter = splitter
        .instantiate(Report(reported.clone()), None::<settings::Map>, None)
        .unwrap_or_else(|error| fail(&format!("instantiating the auto splitter: {error}")));
    let started = Instant::now();
    while started.elapsed() < PATIENCE {
        if let Err(error) = auto_splitter.lock().update() {
            reported
                .lock()
                .unwrap()
                .messages
                .push(format!("the auto splitter trapped: {error}"));
            break;
        }
        if reported.lock().unwrap().variables.contains_key("done") {
            break;
        }
        thread::sleep(auto_splitter.tick_rate());
    }
    drop(auto_splitter);
    let _ = child.kill();
    let _ = child.wait();
    Arc::try_unwrap(reported)
        .ok()
        .expect("the auto splitter is gone")
        .into_inner()
        .unwrap()
}

fn main() {
    let only = std::env::args().nth(1).unwrap_or_default();
    let raw = format!(
        "https://raw.githubusercontent.com/LiveSplit/auto-splitting-test-fixtures/{FIXTURES}"
    );
    let manifest: Manifest = serde_json::from_slice(&fetch(&format!("{raw}/manifest.json")))
        .unwrap_or_else(|error| fail(&format!("manifest.json: {error}")));
    let contract: serde_json::Value =
        serde_json::from_slice(&fetch(&format!("{raw}/unity-fixture.json")))
            .unwrap_or_else(|error| fail(&format!("unity-fixture.json: {error}")));
    let contract = leaves(&contract);
    for path in REQUIRED {
        if !contract.contains_key(*path) {
            fail(&format!("{path} is required but not in the contract"));
        }
    }
    let cache = cache();
    fs::create_dir_all(&cache).expect("creating the cache");

    let runtime = Runtime::new(Config::default()).expect("creating the runtime");
    let splitter = runtime
        .compile(&splitter())
        .expect("compiling the auto splitter");

    let mut failed = 0;
    for entry in on(host(), &manifest.fixtures) {
        let name = format!("{} {}", entry.version, entry.variant);
        if !name.contains(&only) {
            continue;
        }
        let dir = player(entry, &cache);
        let log = cache.join(format!("{}.log", zip_name(entry).trim_end_matches(".zip")));
        let reported = run(&dir, &splitter, &log);
        let problems = check(&reported.variables, &contract, REQUIRED);
        let runtime = reported
            .variables
            .get("runtime")
            .map_or("?", String::as_str);
        let verdict = if problems.is_empty() { "ok" } else { "FAIL" };
        println!("{name:40} {runtime:7} {verdict}");
        if !problems.is_empty() {
            failed += 1;
            for problem in &problems {
                println!("    {problem}");
            }
            for message in &reported.messages {
                println!("    > {message}");
            }
            for (path, value) in &reported.variables {
                println!("    = {path}: {value}");
            }
            // The player's own log says whether it started at all.
            let tail = fs::read_to_string(&log).unwrap_or_default();
            for line in tail
                .lines()
                .rev()
                .take(20)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
            {
                println!("    | {line}");
            }
        }
    }
    if failed > 0 {
        fail(&format!("{failed} players failed"));
    }
}

#[cfg(test)]
mod tests {
    use super::{on, verify, Entry};
    use std::collections::BTreeMap;

    fn entry(variant: &str) -> Entry {
        Entry {
            version: "6000.5.10f1".into(),
            variant: variant.into(),
            asset: format!("https://example.test/unity-6000.5.10f1-{variant}.zip"),
            size: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
        }
    }

    #[test]
    fn picks_the_variants_of_the_host_platform() {
        let entries = [
            entry("win-x64-mono-bdwgc"),
            entry("win-x86-il2cpp-release"),
            entry("linux-x64-mono-bdwgc"),
            entry("mac-mono-bdwgc"),
        ];
        let names = |platform| {
            on(platform, &entries)
                .map(|entry| entry.variant.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names("win"),
            ["win-x64-mono-bdwgc", "win-x86-il2cpp-release"]
        );
        assert_eq!(names("linux"), ["linux-x64-mono-bdwgc"]);
        assert_eq!(names("mac"), ["mac-mono-bdwgc"]);
    }

    #[test]
    fn flattens_the_contract_into_paths() {
        let contract = serde_json::json!({
            "scenes": [{ "path": "Assets/Scenes/Boot.unity", "buildIndex": 0, "roots": [{ "name": "Foo" }] }],
            "classes": { "FixtureData": { "fields": { "floatValue": 3.5, "boolValue": true, "dict": { "one": 1 } } } }
        });
        let leaves = super::leaves(&contract);
        let pairs: Vec<_> = leaves
            .iter()
            .map(|(path, value)| (path.as_str(), value.as_str()))
            .collect();
        assert_eq!(
            pairs,
            [
                ("classes.FixtureData.fields.boolValue", "true"),
                ("classes.FixtureData.fields.dict.one", "1"),
                ("classes.FixtureData.fields.floatValue", "3.5"),
                ("scenes.0.buildIndex", "0"),
                ("scenes.0.path", "Assets/Scenes/Boot.unity"),
                ("scenes.0.roots.0.name", "Foo"),
            ]
        );
    }

    #[test]
    fn a_reported_value_has_to_match_the_contract() {
        let contract = super::leaves(&serde_json::json!({ "a": 1, "b": 2.5, "c": "x" }));
        let reported = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>()
        };
        let required = ["a", "b"];
        assert_eq!(
            super::check(&reported(&[("a", "1"), ("b", "2.5")]), &contract, &required),
            Vec::<String>::new()
        );
        assert_eq!(
            super::check(
                &reported(&[("a", "1"), ("b", "2.50")]),
                &contract,
                &required
            ),
            Vec::<String>::new()
        );
        assert_eq!(
            super::check(&reported(&[("a", "2"), ("b", "2.5")]), &contract, &required),
            ["a is 2, the contract says 1"]
        );
        assert_eq!(
            super::check(&reported(&[("a", "1")]), &contract, &required),
            ["b is missing"]
        );
        assert_eq!(
            super::check(
                &reported(&[("a", "1"), ("b", "2.5"), ("d", "4")]),
                &contract,
                &required
            ),
            ["d is not in the contract"]
        );
    }

    #[test]
    fn refuses_a_zip_the_manifest_does_not_describe() {
        let entry = entry("win-x64-mono-bdwgc");
        assert_eq!(verify(b"abc", &entry), Ok(()));
        assert!(verify(b"abcd", &entry).is_err());
        assert!(verify(b"abd", &entry).is_err());
    }
}
