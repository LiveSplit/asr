//! Runs the Unity fixture players against an auto splitter built on asr
//! and checks what it reads against the fixtures repo's contract.

use std::{
    fs,
    path::{Path, PathBuf},
    process,
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

fn main() {
    let raw = format!(
        "https://raw.githubusercontent.com/LiveSplit/auto-splitting-test-fixtures/{FIXTURES}"
    );
    let manifest: Manifest = serde_json::from_slice(&fetch(&format!("{raw}/manifest.json")))
        .unwrap_or_else(|error| fail(&format!("manifest.json: {error}")));
    let cache = cache();
    fs::create_dir_all(&cache).expect("creating the cache");
    fs::write(
        cache.join("unity-fixture.json"),
        fetch(&format!("{raw}/unity-fixture.json")),
    )
    .expect("writing the contract");

    for entry in on(host(), &manifest.fixtures) {
        let dir = player(entry, &cache);
        println!("{} {} at {}", entry.version, entry.variant, dir.display());
    }
}

#[cfg(test)]
mod tests {
    use super::{on, verify, Entry};

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
    fn refuses_a_zip_the_manifest_does_not_describe() {
        let entry = entry("win-x64-mono-bdwgc");
        assert_eq!(verify(b"abc", &entry), Ok(()));
        assert!(verify(b"abcd", &entry).is_err());
        assert!(verify(b"abd", &entry).is_err());
    }
}
