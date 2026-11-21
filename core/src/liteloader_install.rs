//! LiteLoader installation into a dedicated, caller-owned cache.
//!
//! Trust model, matching what each repository layout actually publishes:
//! - m2 snapshot/release jars: verified against the published `.sha1`
//!   sidecar fetched over HTTPS (repo.mumfrey.com snapshots,
//!   dl.liteloader.com/repo/ releases). The manifest's md5 is stale for
//!   snapshot artifacts, so it never authenticates an m2 jar.
//! - ivy jars (dl.liteloader.com): no published SHA-1 exists; the jar is
//!   verified against the manifest's published md5 (a weak digest, but
//!   fetched over TLS from the same host and confirmed to match the served
//!   artifact), and the installer records its own SHA-1 + size in the cache
//!   manifest for later cache revalidation only. That recorded digest is
//!   never presented as independently pinned trust.
//! - dependency libraries: verified against their host's published `.sha1`
//!   sidecar (libraries.minecraft.net, repo1.maven.org, repo.spongepowered.org;
//!   the latter only responds to a Maven user agent).
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

pub struct InstalledLiteLoader {
    pub profile: Value,
    pub libraries: Vec<PathBuf>,
}

fn client(user_agent: &str) -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .user_agent(user_agent)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(90))
        .build()?)
}

fn get(c: &reqwest::blocking::Client, url: &str, limit: u64) -> Result<Vec<u8>> {
    let response = c.get(url).send()?.error_for_status()?;
    if response.content_length().is_some_and(|n| n > limit) {
        return Err("LiteLoader response exceeds limit".into());
    }
    let mut bytes = Vec::new();
    response.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("LiteLoader response exceeds limit".into());
    }
    Ok(bytes)
}

fn plain_dir(p: &Path) -> Result<()> {
    fs::create_dir_all(p)?;
    let m = fs::symlink_metadata(p)?;
    if !m.is_dir() || m.file_type().is_symlink() {
        return Err("LiteLoader directory must not be a link".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if m.file_attributes() & 0x400 != 0 {
            return Err("LiteLoader reparse point rejected".into());
        }
    }
    Ok(())
}

fn sha1_of(bytes: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    format!("{:x}", Sha1::digest(bytes))
}

fn md5_of(bytes: &[u8]) -> String {
    format!("{:x}", md5::compute(bytes))
}

fn sidecar_sha1(s: &str) -> Result<String> {
    let s = s.trim();
    if s.len() != 40 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid LiteLoader SHA-1 sidecar".into());
    }
    Ok(s.to_ascii_lowercase())
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

/// Parse `<timestamp>`/`<buildNumber>` out of a maven-metadata snapshot
/// document, requiring the document's `<version>` to match.
fn snapshot_coordinates(metadata: &str, version: &str) -> Result<String> {
    let pick = |tag: &str| -> Result<&str> {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        let start = metadata
            .find(&open)
            .ok_or("Missing snapshot metadata tag")?
            + open.len();
        let end = metadata[start..]
            .find(&close)
            .ok_or("Missing snapshot metadata tag")?
            + start;
        Ok(metadata[start..end].trim())
    };
    if pick("version")? != version {
        return Err("Snapshot metadata version mismatch".into());
    }
    let base = version
        .strip_suffix("-SNAPSHOT")
        .ok_or("Not a snapshot version")?;
    if base.is_empty()
        || base
            .bytes()
            .any(|b| !(b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-')))
    {
        return Err("Invalid snapshot base version".into());
    }
    let timestamp = pick("timestamp")?;
    let build_number = pick("buildNumber")?;
    if timestamp.is_empty()
        || !timestamp.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        || build_number.is_empty()
        || !build_number.bytes().all(|b| b.is_ascii_digit())
    {
        return Err("Invalid snapshot coordinates".into());
    }
    Ok(format!("{base}-{timestamp}-{build_number}"))
}

/// Host pinned for a declared dependency coordinate.
fn library_host(name: &str) -> Result<&'static str> {
    if name.starts_with("net.minecraft:launchwrapper:") {
        Ok("https://libraries.minecraft.net/")
    } else if name.starts_with("org.ow2.asm:asm-all:")
        || name.starts_with("net.sf.jopt-simple:jopt-simple:")
    {
        Ok("https://repo1.maven.org/maven2/")
    } else if name.starts_with("org.spongepowered:mixin:") {
        Ok("https://repo.spongepowered.org/maven/")
    } else {
        Err("Unapproved LiteLoader library".into())
    }
}

/// Fetch one verified jar: bytes are returned only after the digest check.
/// `sha1_url` is the published sidecar; `expect_md5` adds the manifest's
/// published md5 as a second required check (ivy) or cross-check (m2).
fn fetch_verified(
    c: &reqwest::blocking::Client,
    url: &str,
    sha1_url: Option<&str>,
    expect_md5: Option<&str>,
) -> Result<Vec<u8>> {
    let bytes = get(c, url, MAX_JAR)?;
    if let Some(sidecar) = sha1_url {
        let published = sidecar_sha1(std::str::from_utf8(&get(c, sidecar, 256)?)?)?;
        if sha1_of(&bytes) != published {
            return Err("LiteLoader jar SHA-1 mismatch".into());
        }
    }
    if let Some(md5) = expect_md5 {
        if md5_of(&bytes) != md5 {
            return Err("LiteLoader jar md5 mismatch".into());
        }
    }
    Ok(bytes)
}

fn profile_for(game: &str) -> Value {
    serde_json::json!({
        "id": format!("liteloader-{game}"),
        "mainClass": super::liteloader::MAIN_CLASS,
        "arguments": {"game": ["--tweakClass", super::liteloader::TWEAK_CLASS]},
    })
}

/// Install only into a caller-owned, separate loader cache. Never changes instances/worlds.
pub fn install(root: &Path, game: &str) -> Result<InstalledLiteLoader> {
    plain_dir(root)?;
    let manifest_path = root.join(format!("liteloader-{game}.json"));
    if manifest_path.exists() {
        return install_cached(root, game, &manifest_path);
    }
    let c = client("AndromedaLauncher/0.0.1")?;
    let maven_ua = client("Apache-Maven/3.9.6")?;
    let manifest: Value =
        serde_json::from_slice(&get(&c, super::liteloader::MANIFEST_URL, MAX_META)?)?;
    let selected = super::liteloader::select(&manifest, game)?;
    let mut files: Vec<(PathBuf, Value)> = Vec::new();
    // The LiteLoader jar itself.
    let jar_dir = root.join("liteloader").join(game);
    plain_dir(&jar_dir)?;
    let jar_dest = jar_dir.join(&selected.file);
    let (jar_url, jar_sha1_url) = match selected.repo {
        super::liteloader::Repo::Ivy => (
            format!(
                "https://dl.liteloader.com/versions/com/mumfrey/liteloader/{game}/{}",
                selected.file
            ),
            None,
        ),
        super::liteloader::Repo::M2Snapshot => {
            let base = format!(
                "https://repo.mumfrey.com/content/repositories/snapshots/com/mumfrey/liteloader/{}/",
                selected.version
            );
            let metadata =
                String::from_utf8(get(&c, &format!("{base}maven-metadata.xml"), MAX_META)?)?;
            let resolved = snapshot_coordinates(&metadata, &selected.version)?;
            (
                format!("{base}liteloader-{resolved}.jar"),
                Some(format!("{base}liteloader-{resolved}.jar.sha1")),
            )
        }
        super::liteloader::Repo::M2Release => {
            // The manifest names dl.liteloader.com/repo/, which 302-redirects
            // to this canonical HTTPS maven path; fetch the target directly.
            let base = format!(
                "https://repo.mumfrey.com/content/repositories/liteloader/com/mumfrey/liteloader/{}/",
                selected.version
            );
            (
                format!("{base}{}", selected.file),
                Some(format!("{base}{}.sha1", selected.file)),
            )
        }
    };
    // Only ivy jars authenticate through the manifest md5; m2 jars are
    // authenticated by their freshly fetched .sha1 sidecar (the snapshot
    // repo rolled after the manifest's md5 was recorded).
    let expect_md5 = match selected.repo {
        super::liteloader::Repo::Ivy => Some(selected.md5.as_str()),
        _ => None,
    };
    let jar_bytes = fetch_verified(&c, &jar_url, jar_sha1_url.as_deref(), expect_md5)?;
    write_atomic(&jar_dest, &jar_bytes)?;
    files.push((
        jar_dest,
        serde_json::json!({
            "path": format!("liteloader/{game}/{}", selected.file),
            "sha1": sha1_of(&jar_bytes),
            "size": jar_bytes.len(),
        }),
    ));
    // Declared dependency libraries.
    for name in &selected.libraries {
        let relative = super::fabric::maven_path(name)?;
        let rel = relative
            .to_str()
            .ok_or("Non-UTF8 Maven path")?
            .replace('\\', "/");
        let host = library_host(name)?;
        let version = name.rsplit(':').next().unwrap_or("");
        let url = if version.strip_suffix("-SNAPSHOT").is_some() {
            let parent = relative
                .parent()
                .ok_or("Missing snapshot parent")?
                .to_str()
                .ok_or("Non-UTF8 Maven path")?
                .replace('\\', "/");
            let metadata = String::from_utf8(get(
                &maven_ua,
                &format!("{host}{parent}/maven-metadata.xml"),
                MAX_META,
            )?)?;
            let artifact = name.split(':').nth(1).unwrap_or("");
            let resolved = snapshot_coordinates(&metadata, version)?;
            format!("{host}{parent}/{artifact}-{resolved}.jar")
        } else {
            format!("{host}{rel}")
        };
        let client_for = if host == "https://repo.spongepowered.org/maven/" {
            &maven_ua
        } else {
            &c
        };
        let bytes = fetch_verified(client_for, &url, Some(&format!("{url}.sha1")), None)?;
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
        write_atomic(&dest, &bytes)?;
        files.push((
            dest,
            serde_json::json!({
                "name": name,
                "path": rel,
                "sha1": sha1_of(&bytes),
                "size": bytes.len(),
            }),
        ));
    }
    let record = serde_json::json!({
        "game": game,
        "selected": {
            "repo": match selected.repo {
                super::liteloader::Repo::Ivy => "ivy",
                super::liteloader::Repo::M2Snapshot => "m2-snapshot",
                super::liteloader::Repo::M2Release => "m2-release",
            },
            "file": selected.file,
            "version": selected.version,
            "md5": selected.md5,
            "libraries": selected.libraries,
        },
        "files": files.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>(),
    });
    write_atomic(&manifest_path, &serde_json::to_vec(&record)?)?;
    Ok(InstalledLiteLoader {
        profile: profile_for(game),
        libraries: files.into_iter().map(|(p, _)| p).collect(),
    })
}

/// Offline path: replay a recorded install, revalidating every file against
/// the SHA-1 + size recorded at download time.
fn install_cached(root: &Path, game: &str, manifest_path: &Path) -> Result<InstalledLiteLoader> {
    let m = fs::symlink_metadata(manifest_path)?;
    if !m.is_file() || m.file_type().is_symlink() || m.len() > MAX_META {
        return Err("Unsafe LiteLoader cache manifest".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if m.file_attributes() & 0x400 != 0 {
            return Err("LiteLoader cache manifest reparse point rejected".into());
        }
    }
    let record: Value = serde_json::from_slice(&fs::read(manifest_path)?)?;
    if record["game"].as_str() != Some(game) {
        return Err("LiteLoader cache manifest game mismatch".into());
    }
    let selected = &record["selected"];
    if !matches!(
        selected["repo"].as_str(),
        Some("ivy") | Some("m2-snapshot") | Some("m2-release")
    ) || selected["file"].as_str().is_none_or(|f| f.is_empty())
        || selected["md5"]
            .as_str()
            .is_none_or(|h| h.len() != 32 || !h.bytes().all(|b| b.is_ascii_hexdigit()))
        || selected["libraries"]
            .as_array()
            .is_none_or(|l| l.is_empty() || l.len() > 8)
    {
        return Err("Invalid LiteLoader cache manifest".into());
    }
    let files = record["files"]
        .as_array()
        .ok_or("Missing LiteLoader cache files")?;
    if files.is_empty() || files.len() > 16 {
        return Err("Invalid LiteLoader cache file count".into());
    }
    let mut out = Vec::new();
    for f in files {
        let rel = f["path"].as_str().ok_or("Missing LiteLoader cache path")?;
        if rel
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
            || rel.contains('\\')
            || rel.contains(':')
        {
            return Err("Invalid LiteLoader cache path".into());
        }
        let sha1 = f["sha1"].as_str().ok_or("Missing LiteLoader cache sha1")?;
        let size = f["size"].as_u64().ok_or("Missing LiteLoader cache size")?;
        if sha1.len() != 40 || !sha1.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Invalid LiteLoader cache sha1".into());
        }
        let dest = root.join(rel);
        if !dest.starts_with(root) || !crate::vanilla::valid(&dest, sha1, size)? {
            return Err("LiteLoader cache file failed verification".into());
        }
        out.push(dest);
    }
    Ok(InstalledLiteLoader {
        profile: profile_for(game),
        libraries: out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_coordinate_parsing() {
        let md = "<metadata><versioning><snapshot><timestamp>20171128.144431</timestamp><buildNumber>4</buildNumber></snapshot><version>1.12.2-SNAPSHOT</version></versioning></metadata>";
        assert_eq!(
            snapshot_coordinates(md, "1.12.2-SNAPSHOT").unwrap(),
            "1.12.2-20171128.144431-4"
        );
        assert!(snapshot_coordinates(md, "1.12.1-SNAPSHOT").is_err());
        assert!(snapshot_coordinates("<version>x</version>", "1.12.2-SNAPSHOT").is_err());
    }
    #[test]
    fn host_pinning() {
        assert_eq!(
            library_host("net.minecraft:launchwrapper:1.12").unwrap(),
            "https://libraries.minecraft.net/"
        );
        assert_eq!(
            library_host("org.spongepowered:mixin:0.7.4-SNAPSHOT").unwrap(),
            "https://repo.spongepowered.org/maven/"
        );
        assert!(library_host("com.example:evil:1.0").is_err());
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    fn fixture() -> (PathBuf, String, String) {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "andromeda-liteloader-cache-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("liteloader/1.7.10")).unwrap();
        fs::create_dir_all(root.join("net/minecraft/launchwrapper/1.11")).unwrap();
        let jar = root.join("liteloader/1.7.10/liteloader-1.7.10_04.jar");
        let lib = root.join("net/minecraft/launchwrapper/1.11/launchwrapper-1.11.jar");
        fs::write(&jar, b"liteloader-bytes").unwrap();
        fs::write(&lib, b"launchwrapper-bytes").unwrap();
        let record = serde_json::json!({
            "game": "1.7.10",
            "selected": {"repo":"ivy","file":"liteloader-1.7.10_04.jar","version":"1.7.10","md5":"0123456789abcdef0123456789abcdef","libraries":["net.minecraft:launchwrapper:1.11"]},
            "files": [
                {"path":"liteloader/1.7.10/liteloader-1.7.10_04.jar","sha1":sha1_of(b"liteloader-bytes"),"size":16},
                {"name":"net.minecraft:launchwrapper:1.11","path":"net/minecraft/launchwrapper/1.11/launchwrapper-1.11.jar","sha1":sha1_of(b"launchwrapper-bytes"),"size":19},
            ],
        });
        fs::write(
            root.join("liteloader-1.7.10.json"),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
        (
            root,
            sha1_of(b"liteloader-bytes"),
            sha1_of(b"launchwrapper-bytes"),
        )
    }

    #[test]
    fn cached_install_revalidates_without_network() {
        let (root, jar_sha1, lib_sha1) = fixture();
        let installed = install(&root, "1.7.10").unwrap();
        assert_eq!(installed.libraries.len(), 2);
        assert_eq!(
            installed.profile["mainClass"].as_str(),
            Some(super::super::liteloader::MAIN_CLASS)
        );
        assert_eq!(
            installed.profile["arguments"]["game"][1].as_str(),
            Some(super::super::liteloader::TWEAK_CLASS)
        );
        assert_eq!(jar_sha1.len(), 40);
        assert_eq!(lib_sha1.len(), 40);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn corrupt_cache_is_rejected_not_overwritten() {
        let (root, _, _) = fixture();
        let jar = root.join("liteloader/1.7.10/liteloader-1.7.10_04.jar");
        fs::write(&jar, b"corrupt!").unwrap();
        assert!(install(&root, "1.7.10").is_err());
        assert_eq!(fs::read(&jar).unwrap(), b"corrupt!");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn wrong_game_cache_is_rejected() {
        let (root, _, _) = fixture();
        // A cache manifest queried under a different game name must not be
        // used: rename the record and require the game mismatch to fail
        // before any network access.
        fs::rename(
            root.join("liteloader-1.7.10.json"),
            root.join("liteloader-1.12.2.json"),
        )
        .unwrap();
        assert!(install(&root, "1.12.2").is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
