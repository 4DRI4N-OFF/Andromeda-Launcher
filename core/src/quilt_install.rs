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

pub struct InstalledQuilt {
    pub profile: Value,
    pub libraries: Vec<PathBuf>,
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
        return Err("Quilt response exceeds limit".into());
    }
    let mut bytes = Vec::new();
    response.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("Quilt response exceeds limit".into());
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
fn sha1(s: &str) -> Result<String> {
    let s = s.trim();
    if s.len() != 40 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid Quilt SHA-1".into());
    }
    Ok(s.to_ascii_lowercase())
}
/// Install only into a caller-owned, separate loader cache. Never changes instances/worlds.
pub fn install(root: &Path, game: &str, loader: &str) -> Result<InstalledQuilt> {
    super::fabric::maven_path(&format!("org.quiltmc:quilt-loader:{loader}"))?;
    super::fabric::maven_path(&format!("net.fabricmc:intermediary:{game}"))?;
    plain_dir(root)?;
    let c = client()?;
    let u = format!("https://meta.quiltmc.org/v3/versions/loader/{game}/{loader}/profile/json");
    let manifest = root.join(format!("profile-{game}-{loader}.json"));
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
    super::quilt::validate_profile(&profile, game, loader)?;
    let mut files = Vec::new();
    let mut verified_libraries = Vec::new();
    for lib in profile["libraries"].as_array().ok_or("Missing libraries")? {
        let relative =
            super::fabric::maven_path(lib["name"].as_str().ok_or("Missing coordinate")?)?;
        let rel = relative
            .to_str()
            .ok_or("Non-UTF8 Maven path")?
            .replace('\\', "/");
        let repo = lib["url"].as_str().ok_or("Missing Quilt repository")?;
        let url = format!("{repo}{rel}");
        let hash = if let Some(h) = lib["sha1"].as_str() {
            sha1(h)?
        } else {
            if cached {
                return Err("Cached loader manifest lacks a digest".into());
            }
            sha1(std::str::from_utf8(&get(&c, &format!("{url}.sha1"), 256)?)?)?
        };
        if cached && lib["size"].as_u64().is_none_or(|n| n == 0) {
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
        } else {
            let bytes = get(&c, &url, MAX_JAR)?;
            use sha1::{Digest, Sha1};
            if format!("{:x}", Sha1::digest(&bytes)) != hash
                || lib["size"]
                    .as_u64()
                    .is_some_and(|n| n != bytes.len() as u64)
            {
                return Err("Quilt library hash/size mismatch".into());
            }
            use std::io::Write;
            let temp = dest.with_extension(format!("{}.part", std::process::id()));
            let mut f = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            let write = (|| -> std::io::Result<()> {
                f.write_all(&bytes)?;
                f.sync_all()?;
                drop(f);
                fs::rename(&temp, &dest)
            })();
            if write.is_err() {
                let _ = fs::remove_file(&temp);
            }
            write?;
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
    Ok(InstalledQuilt {
        profile,
        libraries: files,
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
        let root =
            std::env::temp_dir().join(format!("andromeda-quilt-cache-{}-{id}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let mut libs = Vec::new();
        for n in [
            "org.quiltmc:quilt-loader:0.30.2",
            "org.quiltmc:hashed:1.21.11",
            "net.fabricmc:intermediary:1.21.11",
        ] {
            let path = root.join(crate::fabric::maven_path(n).unwrap());
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, b"verified").unwrap();
            use sha1::{Digest, Sha1};
            libs.push(serde_json::json!({"name":n,"url":if n.starts_with("net.fabricmc:"){"https://maven.fabricmc.net/"}else{"https://maven.quiltmc.org/repository/release/"},"sha1":format!("{:x}",Sha1::digest(b"verified")),"size":8}));
        }
        let profile = serde_json::json!({"id":"quilt-loader-0.30.2-1.21.11","inheritsFrom":"1.21.11","mainClass":"org.quiltmc.loader.impl.launch.knot.KnotClient","libraries":libs});
        (root, profile)
    }
    fn save(root: &Path, p: &Value) {
        fs::write(
            root.join("profile-1.21.11-0.30.2.json"),
            serde_json::to_vec(p).unwrap(),
        )
        .unwrap();
    }
    #[test]
    fn cached_libraries_are_checked_without_network() {
        let (root, p) = fixture();
        save(&root, &p);
        assert!(install(&root, "1.21.11", "0.30.2").is_ok());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn corrupt_cache_is_not_overwritten() {
        let (root, p) = fixture();
        save(&root, &p);
        let jar = root
            .join(crate::fabric::maven_path(p["libraries"][0]["name"].as_str().unwrap()).unwrap());
        fs::write(&jar, b"corrupt!").unwrap();
        assert!(install(&root, "1.21.11", "0.30.2").is_err());
        assert_eq!(fs::read(&jar).unwrap(), b"corrupt!");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn missing_cached_digest_is_rejected() {
        let (root, mut p) = fixture();
        p["libraries"][0].as_object_mut().unwrap().remove("sha1");
        save(&root, &p);
        assert!(install(&root, "1.21.11", "0.30.2").is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
