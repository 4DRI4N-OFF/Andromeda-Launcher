//! Verified vanilla launch with a local profile or a checked Microsoft session.
use crate::vanilla::{allowed, path, valid};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct LaunchPlan {
    pub java: PathBuf,
    pub args: Vec<String>,
    pub game_dir: PathBuf,
}

pub fn offline_uuid(name: &str) -> Result<String> {
    if name.is_empty()
        || name.len() > 16
        || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
    {
        return Err("Local name must be 1-16 ASCII letters, numbers or underscores".into());
    }
    let mut id = md5::compute(format!("OfflinePlayer:{name}")).0;
    id[6] = (id[6] & 0x0f) | 0x30;
    id[8] = (id[8] & 0x3f) | 0x80;
    Ok(id.iter().map(|b| format!("{b:02x}")).collect())
}

fn required<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .ok_or_else(|| format!("Missing {key}").into())
}
fn verified(v: &Value, file: PathBuf) -> Result<PathBuf> {
    let hash = required(v, "sha1")?;
    if hash.len() != 40
        || !hash.bytes().all(|b| b.is_ascii_hexdigit())
        || !valid(&file, hash, v["size"].as_u64().unwrap_or(0))?
    {
        return Err(format!("Missing or changed installed file: {}", file.display()).into());
    }
    Ok(file)
}
fn expand(arg: &str, vars: &HashMap<&str, String>) -> Result<String> {
    let mut out = String::new();
    let mut rest = arg;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        rest = &rest[start + 2..];
        let end = rest.find('}').ok_or("Unclosed launch placeholder")?;
        out.push_str(
            vars.get(&rest[..end])
                .ok_or_else(|| format!("Unsupported launch placeholder: {}", &rest[..end]))?,
        );
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}
fn arguments(list: &Value, vars: &HashMap<&str, String>) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for entry in list.as_array().ok_or("Missing modern launch arguments")? {
        let value = if entry.is_string() {
            entry
        } else if allowed(entry) {
            &entry["value"]
        } else {
            continue;
        };
        if let Some(s) = value.as_str() {
            out.push(expand(s, vars)?);
        } else {
            for s in value.as_array().ok_or("Invalid argument value")? {
                out.push(expand(s.as_str().ok_or("Invalid argument string")?, vars)?);
            }
        }
    }
    Ok(out)
}

// Only x64 Windows DLL entries are published. Never join an archive path onto disk.
fn extract_native(jar: &Path, dest: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(fs::File::open(jar)?)?;
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry.name().to_owned();
        // Validate all entries, including files that will not be extracted.
        path(Path::new("safe"), name.trim_end_matches('/'))?;
        if entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err("Native archive contains a symlink".into());
        }
        if entry.is_dir() || !name.to_ascii_lowercase().ends_with(".dll") {
            continue;
        }
        let parts: Vec<_> = name.split('/').collect();
        if parts
            .iter()
            .any(|p| matches!(*p, "x86" | "arm64" | "aarch64"))
        {
            continue;
        }
        let file = parts.last().ok_or("Missing native filename")?;
        if entry.size() > 128 * 1024 * 1024 {
            return Err("Native entry exceeds size limit".into());
        }
        total += entry.size();
        if total > 512 * 1024 * 1024 {
            return Err("Native archive exceeds size limit".into());
        }
        let mut bytes = Vec::new();
        (&mut entry)
            .take(128 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > 128 * 1024 * 1024 {
            return Err("Expanded native entry exceeds size limit".into());
        }
        let target = path(dest, file)?;
        if target.exists() && fs::read(&target)? != bytes {
            return Err(format!("Conflicting native DLL: {file}").into());
        }
        if !target.exists() {
            fs::write(target, bytes)?;
        }
    }
    Ok(())
}

/// Prepares only the downloaded, tested Windows x64 release. No downloads occur.
pub fn prepare(root: &Path, version: &str, name: &str, memory_mb: u32) -> Result<LaunchPlan> {
    prepare_in_game_dir(root, version, name, memory_mb, &root.join("game"))
}

/// Prepare an isolated, caller-selected game directory with shared verified downloads.
pub fn prepare_in_game_dir(
    root: &Path,
    version: &str,
    name: &str,
    memory_mb: u32,
    game: &Path,
) -> Result<LaunchPlan> {
    prepare_identity(root, version, name, memory_mb, game, None, None)
}

pub fn prepare_authenticated(
    root: &Path,
    version: &str,
    memory_mb: u32,
    game: &Path,
    session: &crate::auth::Session,
) -> Result<LaunchPlan> {
    if session.is_expired() {
        return Err("Microsoft session expired. Sign in again.".into());
    }
    prepare_identity(
        root,
        version,
        &session.profile.name,
        memory_mb,
        game,
        Some(session),
        None,
    )
}
/// Fabric uses the same verified vanilla base and optional checked account.
/// Libraries are installed to a separate cache, never the selected game directory.
pub fn prepare_fabric(
    root: &Path,
    version: &str,
    name: &str,
    memory_mb: u32,
    game: &Path,
    session: Option<&crate::auth::Session>,
    loader: &str,
) -> Result<LaunchPlan> {
    if session.is_some_and(|s| s.is_expired()) {
        return Err("Microsoft session expired. Sign in again.".into());
    }
    let name = session.map(|s| s.profile.name.as_str()).unwrap_or(name);
    prepare_identity(
        root,
        version,
        name,
        memory_mb,
        game,
        session,
        Some(ClientLoader::Fabric(loader)),
    )
}
pub fn prepare_quilt(
    root: &Path,
    version: &str,
    name: &str,
    memory_mb: u32,
    game: &Path,
    session: Option<&crate::auth::Session>,
    loader: &str,
) -> Result<LaunchPlan> {
    if session.is_some_and(|s| s.is_expired()) {
        return Err("Microsoft session expired. Sign in again.".into());
    }
    let name = session.map(|s| s.profile.name.as_str()).unwrap_or(name);
    prepare_identity(
        root,
        version,
        name,
        memory_mb,
        game,
        session,
        Some(ClientLoader::Quilt(loader)),
    )
}
/// Pinned Forge-family launch, using only a dedicated installer cache.
pub fn prepare_upstream(
    root: &Path,
    version: &str,
    name: &str,
    memory_mb: u32,
    game: &Path,
    session: Option<&crate::auth::Session>,
    spec: &crate::upstream_install::Spec,
) -> Result<LaunchPlan> {
    if session.is_some_and(|s| s.is_expired()) {
        return Err("Microsoft session expired. Sign in again.".into());
    }
    let name = session.map(|s| s.profile.name.as_str()).unwrap_or(name);
    let overlay = match spec.kind() {
        crate::upstream_install::Kind::Forge => ClientLoader::Forge(spec.version()),
        crate::upstream_install::Kind::NeoForge => ClientLoader::NeoForge(spec.version()),
    };
    prepare_identity(root, version, name, memory_mb, game, session, Some(overlay))
}
enum ClientLoader<'a> {
    Fabric(&'a str),
    Quilt(&'a str),
    Forge(&'a str),
    NeoForge(&'a str),
}
fn prepare_identity(
    root: &Path,
    version: &str,
    name: &str,
    memory_mb: u32,
    game: &Path,
    session: Option<&crate::auth::Session>,
    fabric_loader: Option<ClientLoader<'_>>,
) -> Result<LaunchPlan> {
    crate::instances::validate_mc_id(version)?;
    if !(512..=8192).contains(&memory_mb) {
        return Err("Memory must be between 512 and 8192 MB".into());
    }
    let local_uuid = offline_uuid(name)?;
    let uuid = session.map(|s| s.profile.id.clone()).unwrap_or(local_uuid);
    let root = std::path::absolute(root)?;
    if !root.is_dir() {
        return Err("Install directory is missing".into());
    }
    let receipt: Value = serde_json::from_slice(&fs::read(root.join("install-receipt.json"))?)?;
    if receipt["version"] != version {
        return Err("Install this version before launching".into());
    }
    let meta = path(&root, &format!("versions/{version}/{version}.json"))?;
    let v: Value = serde_json::from_slice(&fs::read(meta)?)?;
    if v["id"] != version {
        return Err("Unexpected installed version metadata".into());
    }
    // Modern launch requires the arguments object and a Mojang Java runtime.
    // Versions older than that (minecraftArguments string, no javaVersion)
    // are a separate legacy phase and fail here with a clear error.
    if v["arguments"].is_null() {
        return Err(
            "This version predates the modern launch format; legacy versions are not supported yet"
                .into(),
        );
    }
    if v["javaVersion"]["majorVersion"].as_u64().is_none() {
        return Err(
            "This version declares no Java runtime; legacy versions are not supported yet".into(),
        );
    }
    let java_root = path(
        &root.join("runtime"),
        required(&v["javaVersion"], "component")?,
    )?;
    let runtime: Value = serde_json::from_slice(&fs::read(java_root.join("manifest.json"))?)?;
    let java = verified(
        &runtime["files"]["bin/java.exe"]["downloads"]["raw"],
        java_root.join("bin/java.exe"),
    )?;
    let client = verified(
        &v["downloads"]["client"],
        path(&root, &format!("versions/{version}/{version}.jar"))?,
    )?;
    let game_dir = std::path::absolute(game)?;
    fs::create_dir_all(&game_dir)?;
    let natives = root.join("natives").join(version);
    fs::create_dir_all(&natives)?;
    let mut library_directory = root.join("libraries");
    let fabric = fabric_loader
        .map(|loader| match loader {
            ClientLoader::Fabric(loader) => {
                crate::fabric_install::install(&root.join("fabric-libraries"), version, loader)
            }
            ClientLoader::Quilt(loader) => {
                let q =
                    crate::quilt_install::install(&root.join("quilt-libraries"), version, loader)?;
                Ok(crate::fabric_install::InstalledFabric {
                    profile: q.profile,
                    libraries: q.libraries,
                })
            }
            ClientLoader::Forge(loader_version) | ClientLoader::NeoForge(loader_version) => {
                let kind = match loader {
                    ClientLoader::Forge(_) => crate::upstream_install::Kind::Forge,
                    _ => crate::upstream_install::Kind::NeoForge,
                };
                let spec = crate::upstream_install::Spec::new(kind, version, loader_version)?;
                // Keep the original cache folder names for the builds this
                // launcher pinned, so existing verified installs stay valid.
                let cache = if spec == crate::upstream_install::Spec::pinned(kind) {
                    match kind {
                        crate::upstream_install::Kind::Forge => "forge-61.2.5".to_owned(),
                        crate::upstream_install::Kind::NeoForge => "neoforge-21.11.48".to_owned(),
                    }
                } else {
                    let stem = match kind {
                        crate::upstream_install::Kind::Forge => "forge",
                        crate::upstream_install::Kind::NeoForge => "neoforge",
                    };
                    format!("{stem}-{version}-{loader_version}")
                };
                let f = crate::upstream_install::install_spec(
                    &root.join("upstream-installations").join(cache),
                    &java,
                    &root,
                    &spec,
                )?;
                library_directory = f.library_dir;
                Ok(crate::fabric_install::InstalledFabric {
                    profile: f.profile,
                    libraries: f.libraries,
                })
            }
        })
        .transpose()?;
    let mut classpath = fabric
        .as_ref()
        .map(|f| f.libraries.clone())
        .unwrap_or_default();
    let mut native_jars = Vec::new();
    for lib in v["libraries"].as_array().ok_or("Missing libraries")? {
        if !allowed(lib) {
            continue;
        }
        let lib_name = required(lib, "name")?;
        let classifier = lib_name.rsplit(':').next().unwrap_or("");
        if classifier.starts_with("natives-windows-") {
            continue;
        }
        if let Some(a) = lib["downloads"].get("artifact") {
            let file = verified(a, path(&root.join("libraries"), required(a, "path")?)?)?;
            if classifier == "natives-windows" {
                native_jars.push(file.clone());
            }
            classpath.push(file);
        }
        if let Some(key) = lib["natives"]["windows"].as_str() {
            let a = &lib["downloads"]["classifiers"][key.replace("${arch}", "64")];
            native_jars.push(verified(
                a,
                path(&root.join("libraries"), required(a, "path")?)?,
            )?);
        }
    }
    for jar in native_jars {
        extract_native(&jar, &natives)?;
    }
    classpath.push(client);
    let ai = &v["assetIndex"];
    verified(
        ai,
        path(
            &root.join("assets/indexes"),
            &format!("{}.json", required(ai, "id")?),
        )?,
    )?;
    let vars = HashMap::from([
        (
            "library_directory",
            library_directory.to_string_lossy().into_owned(),
        ),
        ("auth_player_name", name.to_owned()),
        ("auth_uuid", uuid),
        (
            "auth_access_token",
            session.map(|s| s.access_token()).unwrap_or("0").into(),
        ),
        (
            "clientid",
            if session.is_some() {
                crate::auth::CLIENT_ID.into()
            } else {
                String::new()
            },
        ),
        ("auth_xuid", String::new()),
        (
            "user_type",
            if session.is_some() {
                "msa".into()
            } else {
                "legacy".into()
            },
        ),
        ("version_name", version.to_owned()),
        ("version_type", "release".into()),
        ("game_directory", game_dir.to_string_lossy().into_owned()),
        (
            "assets_root",
            root.join("assets").to_string_lossy().into_owned(),
        ),
        ("assets_index_name", required(ai, "id")?.to_owned()),
        ("natives_directory", natives.to_string_lossy().into_owned()),
        ("launcher_name", "Andromeda".into()),
        ("launcher_version", env!("CARGO_PKG_VERSION").into()),
        (
            "classpath",
            classpath
                .iter()
                .map(|p| p.to_string_lossy())
                .collect::<Vec<_>>()
                .join(";"),
        ),
    ]);
    let mut args = vec![format!("-Xmx{memory_mb}M")];
    args.extend(arguments(&v["arguments"]["jvm"], &vars)?);
    if let Some(logging) = v["logging"].get("client") {
        let file = verified(
            &logging["file"],
            path(
                &root.join("assets/log_configs"),
                required(&logging["file"], "id")?,
            )?,
        )?;
        args.push(expand(
            required(logging, "argument")?,
            &HashMap::from([("path", file.to_string_lossy().into_owned())]),
        )?);
    }
    if let Some(f) = &fabric {
        if let Some(jvm) = f.profile["arguments"].get("jvm") {
            args.extend(arguments(jvm, &vars)?);
        }
    }
    args.push(
        required(
            fabric.as_ref().map(|f| &f.profile).unwrap_or(&v),
            "mainClass",
        )?
        .to_owned(),
    );
    args.extend(arguments(&v["arguments"]["game"], &vars)?);
    if let Some(f) = &fabric {
        if let Some(game) = f.profile["arguments"].get("game") {
            args.extend(arguments(game, &vars)?);
        }
    }
    Ok(LaunchPlan {
        java,
        args,
        game_dir,
    })
}

pub struct RunningGame {
    child: Child,
    _lock: crate::instances::GameLock,
}
impl RunningGame {
    pub fn id(&self) -> u32 {
        self.child.id()
    }
    pub fn wait(&mut self) -> std::io::Result<ExitStatus> {
        self.child.wait()
    }
}
impl LaunchPlan {
    /// Spawns Java directly, never through a shell; no tokens are logged.
    pub fn spawn(&self) -> Result<RunningGame> {
        if !cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            return Err("Offline launch currently requires Windows x64".into());
        }
        let lock = crate::instances::lock_game(&self.game_dir)?;
        let logs = self.game_dir.join("launcher-logs");
        fs::create_dir_all(&logs)?;
        let stdout = fs::File::create(logs.join("latest-stdout.log"))?;
        let stderr = fs::File::create(logs.join("latest-stderr.log"))?;
        let mut command = Command::new(&self.java);
        command
            .args(&self.args)
            .current_dir(&self.game_dir)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // Java has a GUI; no console window.
        }
        Ok(RunningGame {
            child: command.spawn()?,
            _lock: lock,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn local_names_are_validated() {
        for s in ["", "a b", "../x", "Player;calc", "Ã¡", "abcdefghijklmnopq"] {
            assert!(offline_uuid(s).is_err(), "{s}");
        }
        assert_eq!(
            offline_uuid("Player").unwrap(),
            "a01e3843e5213998958af459800e4d11"
        );
        assert_ne!(
            offline_uuid("Player").unwrap(),
            offline_uuid("player").unwrap()
        );
    }
    #[test]
    fn placeholders_fail_closed_and_keep_spaces_in_one_argument() {
        let vars = HashMap::from([("game_directory", "C:\\Game Folder\\world".into())]);
        assert_eq!(
            expand("${game_directory}", &vars).unwrap(),
            "C:\\Game Folder\\world"
        );
        assert!(expand("${unknown}", &vars).is_err());
        assert!(expand("${game_directory", &vars).is_err());
    }
    #[test]
    fn modern_rules_and_arrays_are_respected() {
        let v = serde_json::json!(["one",{"rules":[{"action":"allow","os":{"name":"linux"}}],"value":"skip"},{"rules":[{"action":"allow","features":{"is_demo_user":true}}],"value":"--demo"},{"rules":[{"action":"allow","os":{"name":"windows"}}],"value":["-cp","path with spaces"]}]);
        assert_eq!(
            arguments(&v, &HashMap::new()).unwrap(),
            vec!["one", "-cp", "path with spaces"]
        );
    }
    #[test]
    fn native_zip_rejects_traversal_and_symlinks() {
        let temp = std::env::temp_dir().join(format!("andromeda-native-{}", std::process::id()));
        fs::create_dir_all(&temp).unwrap();
        let jar = temp.join("native.jar");
        for name in ["../escape.dll", "C:/escape.dll", "..\\escape.dll"] {
            let mut z = zip::ZipWriter::new(fs::File::create(&jar).unwrap());
            z.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            z.write_all(b"x").unwrap();
            z.finish().unwrap();
            assert!(extract_native(&jar, &temp).is_err());
        }
        let mut z = zip::ZipWriter::new(fs::File::create(&jar).unwrap());
        z.add_symlink(
            "link.dll",
            "target",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        z.finish().unwrap();
        assert!(extract_native(&jar, &temp).is_err());
        fs::remove_dir_all(temp).unwrap();
    }
    #[test]
    fn extracts_native_dlls_and_rejects_conflicts() {
        let temp = std::env::temp_dir().join(format!("andromeda-native-ok-{}", std::process::id()));
        fs::create_dir_all(&temp).unwrap();
        let jar = temp.join("native.jar");
        let mut z = zip::ZipWriter::new(fs::File::create(&jar).unwrap());
        for (name, bytes) in [
            ("windows/x64/example.dll", b"x"),
            ("META-INF/MANIFEST.MF", b"m"),
            ("windows/x86/other.dll", b"y"),
        ] {
            z.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            z.write_all(bytes).unwrap();
        }
        z.finish().unwrap();
        extract_native(&jar, &temp).unwrap();
        assert_eq!(fs::read(temp.join("example.dll")).unwrap(), b"x");
        assert!(!temp.join("other.dll").exists());
        assert!(!temp.join("MANIFEST.MF").exists());
        fs::write(temp.join("example.dll"), b"changed").unwrap();
        assert!(extract_native(&jar, &temp).is_err());
        fs::remove_dir_all(temp).unwrap();
    }
    #[test]
    fn unsupported_versions_and_memory_fail_before_accessing_disk() {
        assert!(prepare(Path::new("missing"), "old", "Player", 2048).is_err());
        assert!(prepare(Path::new("missing"), "1.21.11", "Player", 0).is_err());
        assert!(prepare(Path::new("missing"), "1.21.11", "Player", 9000).is_err());
    }
}
