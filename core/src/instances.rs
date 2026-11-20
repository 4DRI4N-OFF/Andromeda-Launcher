//! Recoverable, isolated local instances. Shared downloads stay in vanilla/.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
const SCHEMA: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Loader {
    Fabric { version: String },
    Quilt { version: String },
    Forge { version: String },
    NeoForge { version: String },
}
/// Loader versions are short Maven-style version strings. Compatibility with
/// a specific Minecraft version is checked against the live catalog before
/// creation (see catalog::loader_versions); this check is purely structural
/// so that stored instances stay readable offline.
pub fn validate_loader_version(version: &str) -> Result<()> {
    if version.is_empty()
        || version.len() > 64
        || !version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+'))
    {
        return Err("Invalid loader version".into());
    }
    Ok(())
}
impl Loader {
    fn validate(&self) -> Result<()> {
        let version = match self {
            Self::Fabric { version }
            | Self::Quilt { version }
            | Self::Forge { version }
            | Self::NeoForge { version } => version,
        };
        validate_loader_version(version)
    }
}
/// Minecraft version ids are short release/snapshot ids such as "1.21.11",
/// "26w14a" or "b1.7.3".
pub fn validate_mc_id(version: &str) -> Result<()> {
    if version.is_empty()
        || version.len() > 64
        || !version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
    {
        return Err("Invalid Minecraft version".into());
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Instance {
    pub schema: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub local_name: String,
    pub memory_mb: u32,
    #[serde(default)]
    pub loader: Option<Loader>,
}
fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 80
        || !id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err("Invalid instance identifier".into());
    }
    Ok(())
}
fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
        return Err("Instance name must contain 1-64 visible characters".into());
    }
    Ok(())
}
fn plain_directory(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err("Instance directory must not be a link".into());
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err("Reparse points are not allowed in instances".into());
        }
    }
    Ok(())
}
fn root_ready(root: &Path) -> Result<()> {
    fs::create_dir_all(root)?;
    plain_directory(root)
}
fn unique_id() -> Result<String> {
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(format!(
        "i-{nanos:x}-{:x}-{:x}",
        std::process::id(),
        SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ))
}
fn read_instance(dir: &Path, id: &str) -> Result<Instance> {
    validate_id(id)?;
    plain_directory(dir)?;
    let meta = fs::symlink_metadata(dir.join("instance.json"))?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 16 * 1024 {
        return Err("Invalid instance metadata file".into());
    }
    let instance: Instance = serde_json::from_slice(&fs::read(dir.join("instance.json"))?)?;
    if instance.schema != SCHEMA || instance.id != id {
        return Err("Unsupported or mismatched instance metadata".into());
    }
    validate_mc_id(&instance.version)?;
    validate_name(&instance.name)?;
    if let Some(loader) = &instance.loader {
        loader.validate()?;
    }
    crate::launch::offline_uuid(&instance.local_name)?;
    if !(512..=8192).contains(&instance.memory_mb) {
        return Err("Invalid instance memory".into());
    }
    plain_directory(&dir.join("game"))?;
    Ok(instance)
}
pub fn get(root: &Path, id: &str) -> Result<Instance> {
    validate_id(id)?;
    plain_directory(root)?;
    read_instance(&root.join(id), id)
}
pub fn game_dir(root: &Path, id: &str) -> Result<PathBuf> {
    get(root, id)?;
    Ok(root.join(id).join("game"))
}
pub fn list(root: &Path) -> Result<Vec<Instance>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    plain_directory(root)?;
    let mut instances = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let id = entry.file_name().to_string_lossy().into_owned();
        if id.starts_with('.') || id == "trash" {
            continue;
        }
        instances.push(read_instance(&entry.path(), &id)?);
    }
    instances.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(instances)
}
struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn publish(root: &Path, instance: &Instance, source: Option<&Path>) -> Result<()> {
    let stage_path = root.join(format!(".staging-{}", instance.id));
    fs::create_dir(&stage_path)?;
    let stage = Staging(stage_path);
    fs::create_dir(stage.0.join("game"))?;
    if let Some(source) = source {
        copy_tree(source, &stage.0.join("game"))?;
    }
    fs::write(
        stage.0.join("instance.json"),
        serde_json::to_vec_pretty(instance)?,
    )?;
    fs::rename(&stage.0, root.join(&instance.id))?;
    Ok(())
}
pub fn create(root: &Path, name: &str, local_name: &str, memory_mb: u32) -> Result<Instance> {
    create_for_version(root, name, local_name, memory_mb, "1.21.11", None)
}
/// Creates a fresh isolated instance. Never converts an existing world/instance.
pub fn create_with_loader(
    root: &Path,
    name: &str,
    local_name: &str,
    memory_mb: u32,
    loader: Option<Loader>,
) -> Result<Instance> {
    create_for_version(root, name, local_name, memory_mb, "1.21.11", loader)
}
/// Creates an instance from a selection verified against the catalog. This
/// is the entry point for user-facing selection flows: the combination can
/// only arrive here after catalog::verify_selection confirmed it upstream,
/// so impossible Minecraft/loader pairs are rejected before any instance is
/// written. One loader at most, exactly as selected.
pub fn create_selected(
    root: &Path,
    name: &str,
    local_name: &str,
    memory_mb: u32,
    selection: &crate::catalog::VerifiedSelection,
) -> Result<Instance> {
    let loader = selection.loader().map(|l| {
        let version = l.version.clone();
        match l.kind {
            crate::catalog::LoaderKind::Fabric => Loader::Fabric { version },
            crate::catalog::LoaderKind::Quilt => Loader::Quilt { version },
            // The catalog lists Forge builds combined ("1.21.11-61.2.5");
            // instances store the bare loader build, matching the schema that
            // existing on-disk instances already use.
            crate::catalog::LoaderKind::Forge => {
                let prefix = format!("{}-", selection.mc());
                let bare = version.strip_prefix(&prefix).unwrap_or(&version);
                Loader::Forge {
                    version: bare.to_owned(),
                }
            }
            crate::catalog::LoaderKind::NeoForge => Loader::NeoForge { version },
        }
    });
    create_for_version(root, name, local_name, memory_mb, selection.mc(), loader)
}
/// Creates a fresh isolated instance for any catalog Minecraft version.
/// Callers offering loader selection must first confirm the loader build
/// exists for `version` through catalog::loader_versions.
pub fn create_for_version(
    root: &Path,
    name: &str,
    local_name: &str,
    memory_mb: u32,
    version: &str,
    loader: Option<Loader>,
) -> Result<Instance> {
    validate_mc_id(version)?;
    if let Some(loader) = &loader {
        loader.validate()?;
    }
    validate_name(name)?;
    crate::launch::offline_uuid(local_name)?;
    if !(512..=8192).contains(&memory_mb) {
        return Err("Memory must be between 512 and 8192 MB".into());
    }
    root_ready(root)?;
    let instance = Instance {
        schema: SCHEMA,
        id: unique_id()?,
        name: name.trim().to_owned(),
        version: version.into(),
        local_name: local_name.into(),
        memory_mb,
        loader,
    };
    publish(root, &instance, None)?;
    Ok(instance)
}

/// Held during duplication/trashing, and by the launch process during play.
pub(crate) struct GameLock {
    _file: fs::File,
}
pub(crate) fn lock_game(game: &Path) -> Result<GameLock> {
    plain_directory(game)?;
    let locks = std::env::temp_dir().join("andromeda-game-locks");
    root_ready(&locks)?;
    let canonical = fs::canonicalize(game)?.to_string_lossy().into_owned();
    #[cfg(target_os = "windows")]
    let canonical = canonical.to_lowercase();
    let target = locks.join(format!("{:x}.lock", md5::compute(canonical.as_bytes())));
    if let Ok(meta) = fs::symlink_metadata(&target) {
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err("Invalid game lock file".into());
        }
    }
    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(4);
    } // Allow rename while denying competing read/write opens.
    let file = options
        .open(target)
        .map_err(|e| format!("Game folder is in use or cannot be locked: {e}"))?;
    Ok(GameLock { _file: file })
}
fn copy_tree(source: &Path, dest: &Path) -> Result<()> {
    plain_directory(source)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".andromeda-running.lock" || name == "launcher-logs" {
            continue;
        }
        let meta = fs::symlink_metadata(entry.path())?;
        if meta.file_type().is_symlink() {
            return Err("Cannot duplicate an instance containing links".into());
        }
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err("Cannot duplicate an instance containing reparse points".into());
            }
        }
        let target = dest.join(name);
        if meta.is_dir() {
            fs::create_dir(&target)?;
            copy_tree(&entry.path(), &target)?;
        } else if meta.is_file() {
            fs::copy(entry.path(), target)?;
        } else {
            return Err("Cannot duplicate a special file".into());
        }
    }
    Ok(())
}
pub fn duplicate(root: &Path, id: &str, name: &str) -> Result<Instance> {
    validate_name(name)?;
    let mut instance = get(root, id)?;
    let source = root.join(id).join("game");
    let _guard = lock_game(&source)?;
    instance.id = unique_id()?;
    instance.name = name.trim().into();
    publish(root, &instance, Some(&source))?;
    Ok(instance)
}
/// Move to the launcher's own recoverable trash. Never permanently delete saves.
pub fn trash(root: &Path, id: &str) -> Result<PathBuf> {
    get(root, id)?;
    let _guard = lock_game(&root.join(id).join("game"))?;
    let trash = root.join("trash");
    root_ready(&trash)?;
    let target = trash.join(id);
    if target.exists() {
        return Err("Trash destination already exists".into());
    }
    fs::rename(root.join(id), &target)?;
    Ok(target)
}
pub fn restore(root: &Path, id: &str) -> Result<Instance> {
    validate_id(id)?;
    plain_directory(root)?;
    plain_directory(&root.join("trash"))?;
    let source = root.join("trash").join(id);
    let instance = read_instance(&source, id)?;
    if root.join(id).exists() {
        return Err("An active instance already uses this identifier".into());
    }
    fs::rename(source, root.join(id))?;
    Ok(instance)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temp() -> PathBuf {
        std::env::temp_dir().join(format!("andromeda-instances-{}", unique_id().unwrap()))
    }
    #[test]
    fn instances_have_separate_folders() {
        let root = temp();
        let a = create(&root, "One", "Player", 2048).unwrap();
        let b = create(&root, "Two", "Player", 2048).unwrap();
        assert_ne!(a.id, b.id);
        fs::write(
            game_dir(&root, &a.id).unwrap().join("options.txt"),
            b"first",
        )
        .unwrap();
        assert!(!game_dir(&root, &b.id).unwrap().join("options.txt").exists());
        assert_eq!(list(&root).unwrap().len(), 2);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn duplicate_copies_saves_without_sharing_them() {
        let root = temp();
        let a = create(&root, "One", "Player", 2048).unwrap();
        let game = game_dir(&root, &a.id).unwrap();
        fs::create_dir(game.join("saves")).unwrap();
        fs::write(game.join("saves/world.dat"), b"world").unwrap();
        let b = duplicate(&root, &a.id, "Copy").unwrap();
        let copied = game_dir(&root, &b.id).unwrap().join("saves/world.dat");
        assert_eq!(fs::read(&copied).unwrap(), b"world");
        fs::write(copied, b"new").unwrap();
        assert_eq!(fs::read(game.join("saves/world.dat")).unwrap(), b"world");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn trash_and_restore_preserve_worlds() {
        let root = temp();
        let a = create(&root, "One", "Player", 2048).unwrap();
        fs::write(game_dir(&root, &a.id).unwrap().join("world"), b"save").unwrap();
        let path = trash(&root, &a.id).unwrap();
        assert!(path.join("game/world").is_file());
        assert!(list(&root).unwrap().is_empty());
        restore(&root, &a.id).unwrap();
        assert_eq!(
            fs::read(game_dir(&root, &a.id).unwrap().join("world")).unwrap(),
            b"save"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn identifiers_and_metadata_fail_closed() {
        let root = temp();
        for id in ["../escape", "C:/escape", "CON", "", "."] {
            assert!(get(&root, id).is_err());
        }
        assert!(create(&root, "\n", "Player", 2048).is_err());
        assert!(create(&root, "One", "Bad-Name", 2048).is_err());
        let a = create(&root, "One", "Player", 2048).unwrap();
        fs::write(root.join(&a.id).join("instance.json"), b"{}").unwrap();
        assert!(list(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(target_os = "windows")]
    #[test]
    fn running_folder_cannot_be_duplicated_or_trashed() {
        let root = temp();
        let a = create(&root, "One", "Player", 2048).unwrap();
        let guard = lock_game(&game_dir(&root, &a.id).unwrap()).unwrap();
        assert!(duplicate(&root, &a.id, "Copy").is_err());
        assert!(trash(&root, &a.id).is_err());
        drop(guard);
        assert!(duplicate(&root, &a.id, "Copy").is_ok());
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn duplicate_rejects_symlinks_and_cleans_staging() {
        let root = temp();
        let a = create(&root, "One", "Player", 2048).unwrap();
        std::os::unix::fs::symlink("/tmp", game_dir(&root, &a.id).unwrap().join("outside"))
            .unwrap();
        assert!(duplicate(&root, &a.id, "Copy").is_err());
        assert_eq!(list(&root).unwrap().len(), 1);
        assert!(!fs::read_dir(&root).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".staging")));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn staging_collision_does_not_remove_existing_data() {
        let root = temp();
        let a = create(&root, "One", "Player", 2048).unwrap();
        let mut b = a;
        b.id = unique_id().unwrap();
        let stage = root.join(format!(".staging-{}", b.id));
        fs::create_dir(&stage).unwrap();
        fs::write(stage.join("keep"), b"existing").unwrap();
        assert!(publish(&root, &b, None).is_err());
        assert_eq!(fs::read(stage.join("keep")).unwrap(), b"existing");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn restore_refuses_to_overwrite_an_existing_folder() {
        let root = temp();
        let a = create(&root, "One", "Player", 2048).unwrap();
        trash(&root, &a.id).unwrap();
        fs::create_dir(root.join(&a.id)).unwrap();
        fs::write(root.join(&a.id).join("keep"), b"existing").unwrap();
        assert!(restore(&root, &a.id).is_err());
        assert!(root
            .join("trash")
            .join(&a.id)
            .join("instance.json")
            .exists());
        assert_eq!(
            fs::read(root.join(&a.id).join("keep")).unwrap(),
            b"existing"
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod loader_tests {
    use super::*;
    #[test]
    fn old_instance_json_defaults_to_vanilla() {
        let i: Instance = serde_json::from_str(r#"{"schema":1,"id":"i-test","name":"Old","version":"1.21.11","local_name":"Player","memory_mb":2048}"#).unwrap();
        assert!(i.loader.is_none());
    }
    #[test]
    fn fabric_selection_roundtrips() {
        let loader = Loader::Fabric {
            version: "0.20.1".into(),
        };
        loader.validate().unwrap();
        assert_eq!(
            serde_json::from_str::<Loader>(&serde_json::to_string(&loader).unwrap()).unwrap(),
            loader
        );
    }
    #[test]
    fn unsafe_loader_version_is_rejected() {
        assert!(Loader::Fabric {
            version: "../x".into()
        }
        .validate()
        .is_err());
        assert!(Loader::Forge {
            version: "..\\x".into()
        }
        .validate()
        .is_err());
    }
    #[test]
    fn create_selected_stores_bare_forge_version() {
        let dir =
            std::env::temp_dir().join(format!("andromeda-inst-test-{}-csf", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("mc-manifest.json"),
            r#"{"versions":[{"id":"1.21.11","type":"release","url":"https://piston-meta.mojang.com/x.json","releaseTime":"2025-01-01T00:00:00Z"}]}"#,
        )
        .unwrap();
        fs::write(
            dir.join("forge-maven-meta.json"),
            r#"<?xml version="1.0"?><metadata><versioning><versions><version>1.21.11-61.2.5</version></versions></versioning></metadata>"#,
        )
        .unwrap();
        let dl = crate::vanilla::Downloader::new().unwrap();
        let sel = crate::catalog::verify_selection(
            &dl,
            &dir,
            "1.21.11",
            Some((crate::catalog::LoaderKind::Forge, "1.21.11-61.2.5")),
        )
        .unwrap();
        let root = dir.join("instances");
        let i = create_selected(&root, "Modded", "Player", 2048, &sel).unwrap();
        assert_eq!(
            i.loader,
            Some(Loader::Forge {
                version: "61.2.5".into()
            })
        );
        let _ = fs::remove_dir_all(&dir);
    }
    #[test]
    fn any_structurally_valid_loader_version_is_accepted() {
        // Compatibility with a given Minecraft version is a catalog concern;
        // stored instances must not be tied to a pinned loader build.
        Loader::Forge {
            version: "61.2.5".into(),
        }
        .validate()
        .unwrap();
        Loader::Forge {
            version: "47.4.10".into(),
        }
        .validate()
        .unwrap();
        Loader::NeoForge {
            version: "21.1.197".into(),
        }
        .validate()
        .unwrap();
    }
    #[test]
    fn create_for_version_stores_the_selected_version() {
        let root =
            std::env::temp_dir().join(format!("andromeda-inst-test-{}-cfv", std::process::id()));
        let i = create_for_version(
            &root,
            "Beta",
            "Player",
            2048,
            "b1.7.3",
            Some(Loader::Fabric {
                version: "0.20.1".into(),
            }),
        )
        .unwrap();
        assert_eq!(i.version, "b1.7.3");
        let loaded = get(&root, &i.id).unwrap();
        assert_eq!(loaded.version, "b1.7.3");
        assert_eq!(
            loaded.loader,
            Some(Loader::Fabric {
                version: "0.20.1".into()
            })
        );
        assert!(create_for_version(&root, "Bad", "Player", 2048, "../etc", None).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
