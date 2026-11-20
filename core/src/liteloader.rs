//! LiteLoader metadata validation and build selection.
//!
//! LiteLoader publishes a single manifest, https://dl.liteloader.com/versions/versions.json,
//! covering Minecraft 1.5.2 through 1.12.2. Each game entry resolves through
//! one of three repository layouts:
//!
//! - ivy (1.5.2-1.8): jars under https://dl.liteloader.com/versions/, no
//!   published SHA-1; the manifest's per-build `md5` field is the only
//!   published digest (weak algorithm, fetched over TLS).
//! - m2 snapshot (1.8.9-1.12.2 except 1.9.4/1.10.2): timestamped snapshots on
//!   repo.mumfrey.com with `.sha1` sidecars; the manifest's md5 is stale
//!   there (the snapshot repo rolled after manifest generation), so only the
//!   freshly fetched sidecar authenticates the jar.
//! - m2 release (1.9.4, 1.10.2): plain releases on dl.liteloader.com/repo/
//!   with `.sha1` sidecars.
//!
//! Launch is launchwrapper-based: main class net.minecraft.launchwrapper.Launch
//! plus `--tweakClass com.mumfrey.liteloader.launch.LiteLoaderTweaker`, unlike
//! the Fabric-family mainClass overlays.
use serde_json::Value;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub const MAIN_CLASS: &str = "net.minecraft.launchwrapper.Launch";
pub const TWEAK_CLASS: &str = "com.mumfrey.liteloader.launch.LiteLoaderTweaker";
pub const MANIFEST_URL: &str = "https://dl.liteloader.com/versions/versions.json";

/// Repository layout a game entry resolves through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repo {
    /// dl.liteloader.com/versions/, plain file name, published md5 only.
    Ivy,
    /// repo.mumfrey.com snapshots, timestamped file name, sha1 sidecar.
    M2Snapshot,
    /// repo.mumfrey.com liteloader releases, plain file name, sha1 sidecar.
    M2Release,
}

/// One validated LiteLoader build selected for a game version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selected {
    pub repo: Repo,
    /// Artifact file name, e.g. liteloader-1.7.10_04.jar.
    pub file: String,
    /// Maven version, e.g. 1.12.2-SNAPSHOT or 1.9.4.
    pub version: String,
    /// Published md5 of the LiteLoader jar (manifest-authenticated, weak).
    pub md5: String,
    /// Library coordinates (deduplicated) the build declares plus any
    /// container-level additions (mixin lives at container level).
    pub libraries: Vec<String>,
}

/// Library coordinates LiteLoader builds may declare, and nothing else.
/// The installer pins each to its own host.
fn coordinate_allowed(name: &str) -> bool {
    let versioned = |prefix: &str| {
        name.strip_prefix(prefix)
            .is_some_and(|v| !v.is_empty() && super::fabric::maven_path(name).is_ok())
    };
    versioned("net.minecraft:launchwrapper:")
        || versioned("org.ow2.asm:asm-all:")
        || versioned("net.sf.jopt-simple:jopt-simple:")
        || versioned("org.spongepowered:mixin:")
}

fn md5_valid(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn file_valid(s: &str) -> bool {
    s.starts_with("liteloader-")
        && s.ends_with(".jar")
        && s.len() <= 80
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Select and validate the current build for a game version from the
/// LiteLoader versions manifest.
pub fn select(manifest: &Value, game: &str) -> Result<Selected> {
    let versions = manifest["versions"]
        .as_object()
        .ok_or("Missing LiteLoader versions")?;
    if versions.is_empty() || versions.len() > 64 {
        return Err("Invalid LiteLoader version count".into());
    }
    let entry = versions.get(game).ok_or("LiteLoader has no such game")?;
    let repo = &entry["repo"];
    let repo_type = repo["type"].as_str().ok_or("Missing LiteLoader repo type")?;
    let stream = repo["stream"].as_str().ok_or("Missing LiteLoader stream")?;
    let url = repo["url"].as_str().ok_or("Missing LiteLoader repo url")?;
    let (kind, container_key) = match (repo_type, stream, url) {
        ("ivy", "RELEASE", "http://dl.liteloader.com/versions/")
        | ("ivy", "RELEASE", "https://dl.liteloader.com/versions/") => (Repo::Ivy, "artefacts"),
        ("m2", "SNAPSHOT", "http://repo.mumfrey.com/content/repositories/snapshots/")
        | ("m2", "SNAPSHOT", "https://repo.mumfrey.com/content/repositories/snapshots/") => {
            (Repo::M2Snapshot, "snapshots")
        }
        ("m2", "RELEASE", "http://dl.liteloader.com/repo/")
        | ("m2", "RELEASE", "https://dl.liteloader.com/repo/")
        | ("m2", "RELEASE", "http://repo.mumfrey.com/content/repositories/liteloader/")
        | ("m2", "RELEASE", "https://repo.mumfrey.com/content/repositories/liteloader/") => {
            (Repo::M2Release, "artefacts")
        }
        _ => return Err("Unapproved LiteLoader repository".into()),
    };
    let container = &entry[container_key];
    let builds = container["com.mumfrey:liteloader"]
        .as_object()
        .ok_or("Missing LiteLoader builds")?;
    if builds.is_empty() || builds.len() > 64 {
        return Err("Invalid LiteLoader build count".into());
    }
    // Prefer the manifest's own "latest" pointer; otherwise take the build
    // with the newest numeric timestamp.
    let (key, build) = if let Some(latest) = builds.get("latest") {
        (None, latest)
    } else {
        let (k, b) = builds
            .iter()
            .filter(|(k, _)| *k != "latest")
            .filter_map(|(k, b)| {
                b["timestamp"]
                    .as_str()
                    .and_then(|t| t.parse::<u64>().ok())
                    .map(|t| (t, k, b))
            })
            .max_by_key(|(t, _, _)| *t)
            .map(|(_, k, b)| (k, b))
            .ok_or("No timestamped LiteLoader build")?;
        (Some(k.as_str()), b)
    };
    if build["tweakClass"].as_str() != Some(TWEAK_CLASS) {
        return Err("LiteLoader build has an unexpected tweaker".into());
    }
    let file = build["file"].as_str().ok_or("Missing LiteLoader file")?;
    if !file_valid(file) {
        return Err("Invalid LiteLoader file name".into());
    }
    let md5 = build["md5"]
        .as_str()
        .ok_or("Missing LiteLoader md5")?
        .to_ascii_lowercase();
    if !md5_valid(&md5) {
        return Err("Invalid LiteLoader md5".into());
    }
    // The md5 must bind to a real hash-keyed entry: a "latest" selection
    // requires a sibling key equal to the md5 (the hash-keyed entry may
    // carry the versioned file name while "latest" names the alias); a
    // timestamp selection requires its own key to equal the md5.
    match key {
        Some(k) if k != md5.as_str() => {
            return Err("LiteLoader build key does not match its md5".into())
        }
        None if !builds.contains_key(&md5) => {
            return Err("LiteLoader latest is not bound to a published build".into());
        }
        _ => {}
    }
    let version = build["version"]
        .as_str()
        .ok_or("Missing LiteLoader build version")?;
    match kind {
        Repo::Ivy => {
            // Ivy builds version as "<game>_<build>" ("1.7.10_04").
            let suffix = version
                .strip_prefix(game)
                .ok_or("LiteLoader build version does not match the game")?;
            if !suffix.is_empty()
                && !(suffix.starts_with('_')
                    && suffix.len() > 1
                    && suffix[1..].bytes().all(|b| b.is_ascii_alphanumeric()))
            {
                return Err("LiteLoader build version does not match the game".into());
            }
        }
        Repo::M2Release => {
            if version != game {
                return Err("LiteLoader build version does not match the game".into());
            }
        }
        Repo::M2Snapshot => {
            if version != format!("{game}-SNAPSHOT") {
                return Err("LiteLoader snapshot version does not match the game".into());
            }
        }
    }
    // Dependency set: the build's own libraries plus container-level
    // additions (mixin is declared next to the build map, not inside it).
    let mut libraries: Vec<String> = Vec::new();
    for list in [container["libraries"].as_array(), build["libraries"].as_array()]
        .into_iter()
        .flatten()
    {
        for lib in list {
            let name = lib["name"].as_str().ok_or("Missing LiteLoader library")?;
            if !coordinate_allowed(name) {
                return Err("Unapproved LiteLoader library".into());
            }
            if let Some(u) = lib["url"].as_str() {
                if u != "https://repo.spongepowered.org/maven/"
                    && u != "http://repo.spongepowered.org/maven/"
                {
                    return Err("Unapproved LiteLoader library host".into());
                }
            }
            let name = name.to_owned();
            if !libraries.contains(&name) {
                libraries.push(name);
            }
        }
    }
    if libraries.is_empty() || libraries.len() > 8 {
        return Err("Invalid LiteLoader library count".into());
    }
    Ok(Selected {
        repo: kind,
        file: file.to_owned(),
        version: version.to_owned(),
        md5,
        libraries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ivy_build() -> Value {
        json!({"tweakClass":TWEAK_CLASS,"libraries":[{"name":"net.minecraft:launchwrapper:1.11"},{"name":"org.ow2.asm:asm-all:5.0.3"}],"stream":"RELEASE","file":"liteloader-1.7.10_04.jar","version":"1.7.10_04","md5":"0123456789abcdef0123456789abcdef","timestamp":"1500000000"})
    }
    fn ivy_manifest() -> Value {
        json!({"versions":{"1.7.10":{"repo":{"stream":"RELEASE","type":"ivy","url":"http://dl.liteloader.com/versions/","classifier":"mcpnames"},"artefacts":{"com.mumfrey:liteloader":{"0123456789abcdef0123456789abcdef":ivy_build(),"latest":ivy_build()}}}}})
    }
    fn m2_manifest() -> Value {
        json!({"versions":{"1.12.2":{"repo":{"stream":"SNAPSHOT","type":"m2","url":"http://repo.mumfrey.com/content/repositories/snapshots/","classifier":""},"snapshots":{"libraries":[{"name":"net.minecraft:launchwrapper:1.12"},{"name":"org.spongepowered:mixin:0.7.4-SNAPSHOT","url":"https://repo.spongepowered.org/maven/"}],"com.mumfrey:liteloader":{"1420785ecbfed5aff4a586c5c9dd97eb":{"tweakClass":TWEAK_CLASS,"libraries":[{"name":"net.minecraft:launchwrapper:1.12"},{"name":"org.ow2.asm:asm-all:5.2"}],"stream":"SNAPSHOT","file":"liteloader-1.12.2-SNAPSHOT.jar","version":"1.12.2-SNAPSHOT","md5":"1420785ecbfed5aff4a586c5c9dd97eb","timestamp":"1511880271"},"latest":{"tweakClass":TWEAK_CLASS,"libraries":[{"name":"net.minecraft:launchwrapper:1.12"},{"name":"org.ow2.asm:asm-all:5.2"}],"stream":"SNAPSHOT","file":"liteloader-1.12.2-SNAPSHOT.jar","version":"1.12.2-SNAPSHOT","md5":"1420785ecbfed5aff4a586c5c9dd97eb","timestamp":"1511880271"}}}}}})
    }

    #[test]
    fn ivy_selection() {
        let s = select(&ivy_manifest(), "1.7.10").unwrap();
        assert_eq!(s.repo, Repo::Ivy);
        assert_eq!(s.file, "liteloader-1.7.10_04.jar");
        assert_eq!(s.version, "1.7.10_04");
        assert_eq!(
            s.libraries,
            vec![
                "net.minecraft:launchwrapper:1.11".to_owned(),
                "org.ow2.asm:asm-all:5.0.3".to_owned()
            ]
        );
    }
    #[test]
    fn m2_snapshot_merges_container_libraries() {
        let s = select(&m2_manifest(), "1.12.2").unwrap();
        assert_eq!(s.repo, Repo::M2Snapshot);
        assert_eq!(s.version, "1.12.2-SNAPSHOT");
        assert!(s.libraries.contains(&"org.spongepowered:mixin:0.7.4-SNAPSHOT".to_owned()));
        assert!(s.libraries.contains(&"org.ow2.asm:asm-all:5.2".to_owned()));
    }
    #[test]
    fn rejects_unknown_game() {
        assert!(select(&ivy_manifest(), "1.13").is_err());
    }
    #[test]
    fn rejects_unapproved_repo() {
        let mut m = ivy_manifest();
        m["versions"]["1.7.10"]["repo"]["url"] = json!("https://example.invalid/");
        assert!(select(&m, "1.7.10").is_err());
    }
    #[test]
    fn rejects_wrong_tweaker() {
        let mut m = ivy_manifest();
        m["versions"]["1.7.10"]["artefacts"]["com.mumfrey:liteloader"]["latest"]["tweakClass"] =
            json!("other.Tweaker");
        assert!(select(&m, "1.7.10").is_err());
    }
    #[test]
    fn rejects_key_md5_mismatch() {
        let mut m = ivy_manifest();
        m["versions"]["1.7.10"]["artefacts"]["com.mumfrey:liteloader"]["latest"]["md5"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert!(select(&m, "1.7.10").is_err());
    }
    #[test]
    fn rejects_unapproved_library() {
        let mut m = ivy_manifest();
        m["versions"]["1.7.10"]["artefacts"]["com.mumfrey:liteloader"]["latest"]["libraries"][0]
            ["name"] = json!("com.example:evil:1.0");
        assert!(select(&m, "1.7.10").is_err());
    }
    #[test]
    fn snapshot_version_must_match_game() {
        let mut m = m2_manifest();
        m["versions"]["1.12.2"]["snapshots"]["com.mumfrey:liteloader"]["latest"]["version"] =
            json!("1.12.1-SNAPSHOT");
        assert!(select(&m, "1.12.2").is_err());
    }
    #[test]
    fn falls_back_to_newest_timestamp_without_latest() {
        let mut m = ivy_manifest();
        let builds = m["versions"]["1.7.10"]["artefacts"]["com.mumfrey:liteloader"]
            .as_object_mut()
            .unwrap();
        builds.remove("latest");
        let mut older = ivy_build();
        older["file"] = json!("liteloader-1.7.10_03.jar");
        older["md5"] = json!("fedcba9876543210fedcba9876543210");
        older["timestamp"] = json!("1400000000");
        builds.insert("fedcba9876543210fedcba9876543210".into(), older);
        let s = select(&m, "1.7.10").unwrap();
        assert_eq!(s.file, "liteloader-1.7.10_04.jar");
    }
}
