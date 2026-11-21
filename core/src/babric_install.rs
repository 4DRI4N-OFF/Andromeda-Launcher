//! Babric installation into a dedicated, caller-owned cache.
//! Same verified-download rules as the Fabric installer; library hosts are
//! restricted to maven.fabricmc.net, maven.glass-launcher.net,
//! repo1.maven.org and libraries.minecraft.net, all served directly.
use serde_json::Value;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
const MAX_META: u64 = 2 * 1024 * 1024;
const MAX_JAR: u64 = 32 * 1024 * 1024;

pub struct InstalledBabric {
    pub profile: Value,
    pub libraries: Vec<PathBuf>,
    /// OS-specific natives jars for classifier-only artifacts (no main jar
    /// exists upstream, e.g. patched LWJGL platform). Kept separate from the
    /// classpath; the launch phase extracts them.
    pub natives: Vec<PathBuf>,
}
fn client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .user_agent("AndromedaLauncher/0.0.1")
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(90))
        .build()?)
}
fn get(c: &reqwest::blocking::Client, url: &str, limit: u64) -> Result<Vec<u8>> {
    let response = c.get(url).send()?.error_for_status()?;
    if response.content_length().is_some_and(|n| n > limit) {
        return Err("Babric response exceeds limit".into());
    }
    let mut bytes = Vec::new();
    response.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("Babric response exceeds limit".into());
    }
    Ok(bytes)
}
fn plain_dir(p: &Path) -> Result<()> {
    fs::create_dir_all(p)?;
    let m = fs::symlink_metadata(p)?;
    if !m.is_dir() || m.file_type().is_symlink() {
        return Err("Loader directory must not be a link".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if m.file_attributes() & 0x400 != 0 {
            return Err("Loader reparse point rejected".into());
        }
    }
    Ok(())
}

fn is_not_found(e: &(dyn std::error::Error + Send + Sync + 'static)) -> bool {
    e.downcast_ref::<reqwest::Error>()
        .and_then(|r| r.status())
        .is_some_and(|s| s.as_u16() == 404)
}
/// LWJGL2's platform artifact publishes only per-OS natives classifiers, no
/// main jar (same as Mojang's own metadata). Download and verify the natives
/// jar for this OS instead; anything else missing stays a hard error.
fn natives_classifier() -> Result<&'static str> {
    match std::env::consts::OS {
        "windows" => Ok("natives-windows"),
        "linux" => Ok("natives-linux"),
        "macos" => Ok("natives-osx"),
        other => Err(format!("No Babric natives classifier for {other}").into()),
    }
}
fn natives_relative(relative: &Path) -> Result<PathBuf> {
    let dir = relative.parent().ok_or("Missing library parent")?;
    let stem = relative
        .file_name()
        .and_then(|f| f.to_str())
        .and_then(|f| f.strip_suffix(".jar"))
        .ok_or("Unexpected library file name")?;
    let mut out = dir.to_path_buf();
    out.push(format!("{stem}-{}.jar", natives_classifier()?));
    Ok(out)
}
fn fetch_natives_instead(
    c: &reqwest::blocking::Client,
    host: &str,
    relative: &Path,
) -> Result<(PathBuf, Vec<u8>, String)> {
    let out = natives_relative(relative)?;
    let rel = out
        .to_str()
        .ok_or("Non-UTF8 Maven path")?
        .replace('\\', "/");
    let url = format!("{host}{rel}");
    let hash = sha1(std::str::from_utf8(&get(c, &format!("{url}.sha1"), 256)?)?)?;
    let bytes = get(c, &url, MAX_JAR)?;
    use sha1::{Digest, Sha1};
    if format!("{:x}", Sha1::digest(&bytes)) != hash {
        return Err("Babric natives hash mismatch".into());
    }
    Ok((out, bytes, hash))
}
fn write_atomic(dest: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let temp = dest.with_extension(format!("{}.part", std::process::id()));
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let write = (|| -> std::io::Result<()> {
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        fs::rename(&temp, dest)
    })();
    if write.is_err() {
        let _ = fs::remove_file(&temp);
    }
    write?;
    Ok(())
}
fn sha1(s: &str) -> Result<String> {
    let s = s.trim();
    if s.len() != 40 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid Babric SHA-1".into());
    }
    Ok(s.to_ascii_lowercase())
}
/// Install only into a caller-owned, separate loader cache. Never changes instances/worlds.
pub fn install(root: &Path, game: &str, loader: &str) -> Result<InstalledBabric> {
    super::fabric::maven_path(&format!("net.fabricmc:fabric-loader:{loader}"))?;
    super::fabric::maven_path(&format!("babric:intermediary-upstream:{game}"))?;
    plain_dir(root)?;
    let c = client()?;
    let u = format!(
        "https://meta.babric.glass-launcher.net/v2/versions/loader/{game}/{loader}/profile/json"
    );
    let manifest = root.join(format!("profile-babric-{game}-{loader}.json"));
    let cached = manifest.exists();
    let mut profile: Value = if cached {
        let m = fs::symlink_metadata(&manifest)?;
        if !m.is_file() || m.file_type().is_symlink() || m.len() > MAX_META {
            return Err("Unsafe loader manifest".into());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if m.file_attributes() & 0x400 != 0 {
                return Err("Loader manifest reparse point rejected".into());
            }
        }
        serde_json::from_slice(&fs::read(&manifest)?)?
    } else {
        serde_json::from_slice(&get(&c, &u, MAX_META)?)?
    };
    super::babric::validate_profile(&profile, game, loader)?;
    let mut files = Vec::new();
    let mut natives = Vec::new();
    let mut verified_libraries = Vec::new();
    for lib in profile["libraries"].as_array().ok_or("Missing libraries")? {
        // Glass Launcher's maven publishes no .sha1 sidecars for some
        // artifacts; Babric's meta embeds Mojang-style download blocks
        // instead. Normalize the inline artifact digest into the fields the
        // verified-download flow expects.
        let mut lib = lib.clone();
        let relative =
            super::fabric::maven_path(lib["name"].as_str().ok_or("Missing coordinate")?)?;
        let rel = relative
            .to_str()
            .ok_or("Non-UTF8 Maven path")?
            .replace('\\', "/");
        // Every Babric host serves artifacts directly (verified: no
        // redirects); downloads stay pinned to the approved host.
        let host = match lib["url"].as_str() {
            Some("https://maven.fabricmc.net/") => "https://maven.fabricmc.net/",
            Some("https://maven.glass-launcher.net/babric/") => {
                "https://maven.glass-launcher.net/babric/"
            }
            Some("https://repo1.maven.org/maven2/") => "https://repo1.maven.org/maven2/",
            Some("https://libraries.minecraft.net/") => "https://libraries.minecraft.net/",
            _ => return Err("Unapproved Babric library host".into()),
        };
        let url = format!("{host}{rel}");
        // Natives-classifier entries (patched LWJGL platform) name the per-OS
        // artifact and its digest inside the profile. Verify the path against
        // the coordinate, then fetch and check it like any other download.
        if lib["nativesOnly"] != Value::Bool(true) && lib["downloads"]["classifiers"].is_object() {
            let classifier = natives_classifier()?;
            let entry = &lib["downloads"]["classifiers"][classifier];
            let expected = natives_relative(&relative)?;
            let rel_n = expected
                .to_str()
                .ok_or("Non-UTF8 Maven path")?
                .replace('\\', "/");
            if entry["path"].as_str() != Some(rel_n.as_str()) {
                return Err("Babric natives path does not match the coordinate".into());
            }
            let nhash = sha1(
                entry["sha1"]
                    .as_str()
                    .ok_or("Babric natives lack a digest")?,
            )?;
            let nsize = entry["size"].as_u64().ok_or("Babric natives lack a size")?;
            let ndest = root.join(&expected);
            let mut parent = root.to_path_buf();
            for part in expected
                .parent()
                .ok_or("Missing library parent")?
                .components()
            {
                parent.push(part);
                plain_dir(&parent)?;
            }
            if let Ok(m) = fs::symlink_metadata(&ndest) {
                if !m.is_file() || m.file_type().is_symlink() {
                    return Err("Loader file must not be a link".into());
                }
                if !crate::vanilla::valid(&ndest, &nhash, nsize)? {
                    return Err(
                        "Existing Babric natives failed verification; cache not overwritten".into(),
                    );
                }
            } else {
                let bytes = get(&c, &format!("{host}{rel_n}"), MAX_JAR)?;
                use sha1::{Digest, Sha1};
                if format!("{:x}", Sha1::digest(&bytes)) != nhash || bytes.len() as u64 != nsize {
                    return Err("Babric natives hash/size mismatch".into());
                }
                write_atomic(&ndest, &bytes)?;
            }
            let mut checked = lib.clone();
            checked["nativesOnly"] = Value::Bool(true);
            checked["sha1"] = Value::String(nhash);
            checked["size"] = Value::from(nsize);
            verified_libraries.push(checked);
            natives.push(ndest);
            continue;
        }
        if lib["sha1"].as_str().is_none() && lib["downloads"]["artifact"]["sha1"].is_string() {
            let artifact = lib["downloads"]["artifact"].clone();
            if artifact["path"].as_str() != Some(rel.as_str()) {
                return Err("Babric artifact path does not match the coordinate".into());
            }
            lib["sha1"] =
                Value::String(artifact["sha1"].as_str().expect("checked above").to_owned());
            lib["size"] = Value::from(
                artifact["size"]
                    .as_u64()
                    .ok_or("Babric artifact lacks a size")?,
            );
        }
        if lib["nativesOnly"] == Value::Bool(true) {
            // Classifier-only artifact recorded by an earlier run: re-verify
            // the natives jar against the recorded digest, never the network.
            let hash = sha1(lib["sha1"].as_str().ok_or("Cached natives lack a digest")?)?;
            let size = lib["size"].as_u64().ok_or("Cached natives lack a size")?;
            let dest = root.join(natives_relative(&relative)?);
            let m = fs::symlink_metadata(&dest)
                .map_err(|_| "Cached Babric natives missing".to_owned())?;
            if !m.is_file() || m.file_type().is_symlink() {
                return Err("Loader file must not be a link".into());
            }
            if !crate::vanilla::valid(&dest, &hash, size)? {
                return Err("Cached Babric natives failed verification".into());
            }
            natives.push(dest);
            verified_libraries.push(lib.clone());
            continue;
        }
        let mut classifier_only = false;
        let hash = if let Some(h) = lib["sha1"].as_str() {
            sha1(h)?
        } else {
            if cached {
                return Err("Cached loader manifest lacks a digest".into());
            }
            match get(&c, &format!("{url}.sha1"), 256) {
                Ok(b) => sha1(std::str::from_utf8(&b)?)?,
                Err(e) if is_not_found(e.as_ref()) => {
                    classifier_only = true;
                    String::new()
                }
                Err(e) => return Err(e),
            }
        };
        if cached && lib["size"].as_u64().is_none_or(|n| n == 0) && !classifier_only {
            return Err("Cached loader manifest lacks a valid size".into());
        }
        let mut parent = root.to_path_buf();
        for part in relative
            .parent()
            .ok_or("Missing library parent")?
            .components()
        {
            parent.push(part);
            plain_dir(&parent)?;
        }
        let dest = root.join(&relative);
        if let Ok(m) = fs::symlink_metadata(&dest) {
            if !m.is_file() || m.file_type().is_symlink() {
                return Err("Loader file must not be a link".into());
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if m.file_attributes() & 0x400 != 0 {
                    return Err("Loader reparse file rejected".into());
                }
            }
            if !crate::vanilla::valid(&dest, &hash, lib["size"].as_u64().unwrap_or(0))? {
                return Err(
                    "Existing loader library failed verification; cache not overwritten".into(),
                );
            }
        } else if classifier_only {
            let (rel_natives, bytes, nhash) = fetch_natives_instead(&c, host, &relative)?;
            let ndest = root.join(&rel_natives);
            write_atomic(&ndest, &bytes)?;
            let mut checked = lib.clone();
            checked["nativesOnly"] = Value::Bool(true);
            checked["sha1"] = Value::String(nhash);
            checked["size"] = Value::from(bytes.len() as u64);
            verified_libraries.push(checked);
            natives.push(ndest);
            continue;
        } else {
            let bytes = match get(&c, &url, MAX_JAR) {
                Ok(b) => b,
                Err(e) if is_not_found(e.as_ref()) => {
                    let (rel_natives, nbytes, nhash) = fetch_natives_instead(&c, host, &relative)?;
                    let ndest = root.join(&rel_natives);
                    write_atomic(&ndest, &nbytes)?;
                    let mut checked = lib.clone();
                    checked["nativesOnly"] = Value::Bool(true);
                    checked["sha1"] = Value::String(nhash);
                    checked["size"] = Value::from(nbytes.len() as u64);
                    verified_libraries.push(checked);
                    natives.push(ndest);
                    continue;
                }
                Err(e) => return Err(e),
            };
            use sha1::{Digest, Sha1};
            if format!("{:x}", Sha1::digest(&bytes)) != hash
                || lib["size"]
                    .as_u64()
                    .is_some_and(|n| n != bytes.len() as u64)
            {
                return Err("Babric library hash/size mismatch".into());
            }
            write_atomic(&dest, &bytes)?;
        }
        let mut checked = lib.clone();
        checked["sha1"] = Value::String(hash);
        checked["size"] = Value::from(fs::metadata(&dest)?.len());
        verified_libraries.push(checked);
        files.push(dest);
    }
    if !cached {
        profile["libraries"] = Value::Array(verified_libraries);
        use std::io::Write;
        let temp = manifest.with_extension(format!("{}.part", std::process::id()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let write = (|| -> std::io::Result<()> {
            file.write_all(&serde_json::to_vec(&profile)?)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp, &manifest)
        })();
        if write.is_err() {
            let _ = fs::remove_file(&temp);
        }
        write?;
    }
    Ok(InstalledBabric {
        profile,
        libraries: files,
        natives,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sidecar_hash_strict() {
        assert!(sha1("<!DOCTYPE html>").is_err());
        assert!(sha1("0123456789012345678901234567890123456789 extra").is_err());
        assert_eq!(
            sha1("  ABCDEF0123456789abcdef0123456789abcdef01\n").unwrap(),
            "abcdef0123456789abcdef0123456789abcdef01"
        );
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    fn fixture() -> (PathBuf, Value) {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "andromeda-fabric-cache-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let mut libs = Vec::new();
        for n in [
            "net.fabricmc:fabric-loader:0.20.1",
            "babric:intermediary-upstream:b1.7.3",
        ] {
            let path = root.join(crate::fabric::maven_path(n).unwrap());
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, b"verified").unwrap();
            use sha1::{Digest, Sha1};
            let host = if n.starts_with("babric:") {
                "https://maven.glass-launcher.net/babric/"
            } else {
                "https://maven.fabricmc.net/"
            };
            libs.push(serde_json::json!({"name":n,"url":host,"sha1":format!("{:x}",Sha1::digest(b"verified")),"size":8}));
        }
        let profile = serde_json::json!({"id":"fabric-loader-0.20.1-b1.7.3","inheritsFrom":"b1.7.3","mainClass":"net.fabricmc.loader.impl.launch.knot.KnotClient","libraries":libs});
        (root, profile)
    }
    fn save(root: &Path, p: &Value) {
        fs::write(
            root.join("profile-babric-b1.7.3-0.20.1.json"),
            serde_json::to_vec(p).unwrap(),
        )
        .unwrap();
    }
    #[test]
    fn cached_libraries_are_checked_without_network() {
        let (root, p) = fixture();
        save(&root, &p);
        assert!(install(&root, "b1.7.3", "0.20.1").is_ok());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn corrupt_cache_is_not_overwritten() {
        let (root, p) = fixture();
        save(&root, &p);
        let jar = root
            .join(crate::fabric::maven_path(p["libraries"][0]["name"].as_str().unwrap()).unwrap());
        fs::write(&jar, b"corrupt!").unwrap();
        assert!(install(&root, "b1.7.3", "0.20.1").is_err());
        assert_eq!(fs::read(&jar).unwrap(), b"corrupt!");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn missing_cached_digest_is_rejected() {
        let (root, mut p) = fixture();
        p["libraries"][0].as_object_mut().unwrap().remove("sha1");
        save(&root, &p);
        assert!(install(&root, "b1.7.3", "0.20.1").is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
