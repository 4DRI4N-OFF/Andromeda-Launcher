//! Per-instance mod registry and file operations.
//!
//! Every instance keeps its mods in `<instance>/game/mods/`. Andromeda
//! tracks what it installed in `.andromeda-mods.json` inside that folder
//! (hidden file, never passed to the game). Disabling renames the archive
//! with a `.disabled` suffix, which every supported loader ignores.
//!
//! Files the user dropped in manually are not touched: they show up as
//! `untracked` so the UI can list them without claiming to manage them.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::modrinth::{self, ModFile, Planned};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Registry file name inside the instance's mods directory.
pub const REGISTRY: &str = ".andromeda-mods.json";
const DISABLED_SUFFIX: &str = ".disabled";
const MAX_ENTRIES: usize = 512;

fn plain_name(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('.')
        && s.len() <= 140
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+'))
}

fn hex128(s: &str) -> bool {
    s.len() == 128 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// One mod Andromeda installed into an instance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModEntry {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub version_id: String,
    pub version_number: String,
    /// Active file name inside mods/ (without any .disabled suffix).
    pub filename: String,
    pub sha512: String,
    pub size: u64,
    pub mc: String,
    /// Modrinth loader facet this build was selected for.
    pub loader: String,
    pub enabled: bool,
    /// Project that required this mod, when installed as a dependency.
    pub required_by: Option<String>,
}

impl ModEntry {
    fn validate(&self) -> Result<()> {
        if !plain_name(&self.filename)
            || !(self.filename.ends_with(".jar") || self.filename.ends_with(".litemod"))
        {
            return Err("Registry entry has unsafe file name".into());
        }
        if !hex128(&self.sha512) {
            return Err("Registry entry has invalid digest".into());
        }
        crate::instances::validate_mc_id(&self.mc)?;
        Ok(())
    }

    /// File name as present on disk right now.
    pub fn disk_name(&self) -> String {
        if self.enabled {
            self.filename.clone()
        } else {
            format!("{}{DISABLED_SUFFIX}", self.filename)
        }
    }
}

/// The per-instance registry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Registry {
    pub mods: Vec<ModEntry>,
}

impl Registry {
    pub fn get(&self, project_id: &str) -> Option<&ModEntry> {
        self.mods.iter().find(|m| m.project_id == project_id)
    }

    /// Registry lookup closure for `modrinth::plan_install`.
    pub fn installed(&self, project_id: &str) -> Option<modrinth::Installed> {
        self.get(project_id).map(|m| modrinth::Installed {
            version_id: m.version_id.clone(),
            mc: m.mc.clone(),
            loader: m.loader.clone(),
            enabled: m.enabled,
        })
    }
}

/// The instance's mods directory (`<game>/mods`). Not created by reading.
pub fn mods_dir(game: &Path) -> Result<PathBuf> {
    if !game.is_dir() {
        return Err("Instance game directory missing".into());
    }
    let dir = game.join("mods");
    reject_link(&dir)?;
    Ok(dir)
}

/// Refuse to act through symlinks or reparse points: a managed path that
/// points elsewhere would make us write or delete outside the instance.
fn reject_link(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => {
            Err("Refusing to follow a symlink inside the instance".into())
        }
        _ => Ok(()),
    }
}

/// True when an enabled installed mod lists `project_id` as its reason
/// for being here, i.e. something active still needs it.
fn required_by_enabled(reg: &Registry, project_id: &str) -> Option<String> {
    let entry = reg.get(project_id)?;
    let owner = entry.required_by.as_ref()?;
    if reg.get(owner).map(|m| m.enabled).unwrap_or(false) {
        Some(owner.clone())
    } else {
        None
    }
}

/// Load the registry; a missing registry means no managed mods.
pub fn load(game: &Path) -> Result<Registry> {
    let path = mods_dir(game)?.join(REGISTRY);
    let body = match fs::read_to_string(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Registry::default()),
        Err(e) => return Err(e.into()),
    };
    if body.len() > 1024 * 1024 {
        return Err("Mod registry too large".into());
    }
    let v: serde_json::Value = serde_json::from_str(&body)?;
    let list = v["mods"].as_array().ok_or("Mod registry missing mods")?;
    if list.len() > MAX_ENTRIES {
        return Err("Too many registry entries".into());
    }
    let mut reg = Registry::default();
    for item in list {
        let entry: ModEntry = serde_json::from_value(item.clone())?;
        entry.validate()?;
        if reg.get(&entry.project_id).is_some() {
            return Err("Duplicate project in mod registry".into());
        }
        reg.mods.push(entry);
    }
    Ok(reg)
}

/// Persist the registry atomically (temp file + rename, like the
/// installer caches, so a crash never leaves a torn registry).
pub fn save(game: &Path, reg: &Registry) -> Result<()> {
    if reg.mods.len() > MAX_ENTRIES {
        return Err("Too many registry entries".into());
    }
    for m in &reg.mods {
        m.validate()?;
    }
    let dir = mods_dir(game)?;
    fs::create_dir_all(&dir)?;
    let body = serde_json::to_string_pretty(&serde_json::json!({ "mods": reg.mods }))?;
    let tmp = dir.join(format!(".registry-{}.tmp", std::process::id()));
    fs::write(&tmp, body)?;
    fs::rename(&tmp, dir.join(REGISTRY))?;
    Ok(())
}

/// Record a resolved install step (after its file is on disk).
pub fn record(reg: &mut Registry, planned: &Planned, mc: &str, facet: &str) -> Result<()> {
    let entry = ModEntry {
        project_id: planned.project_id.clone(),
        slug: planned.project_id.clone(),
        title: planned.title.clone(),
        version_id: planned.version_id.clone(),
        version_number: planned.version_number.clone(),
        filename: planned.file.filename.clone(),
        sha512: planned.file.sha512.clone(),
        size: planned.file.size,
        mc: mc.into(),
        loader: facet.into(),
        enabled: true,
        required_by: planned.required_by.clone(),
    };
    entry.validate()?;
    reg.mods.retain(|m| m.project_id != entry.project_id);
    reg.mods.push(entry);
    Ok(())
}

/// Enable or disable a managed mod by renaming its archive. Refuses to
/// clobber an existing file.
pub fn set_enabled(game: &Path, reg: &mut Registry, project_id: &str, enabled: bool) -> Result<()> {
    let idx = reg
        .mods
        .iter()
        .position(|m| m.project_id == project_id)
        .ok_or("Mod is not managed in this instance")?;
    let (from, to) = {
        let m = &reg.mods[idx];
        if m.enabled == enabled {
            return Ok(());
        }
        if !enabled {
            if let Some(owner) = required_by_enabled(reg, project_id) {
                return Err(format!(
                    "Cannot disable: project {owner} is enabled and requires this mod"
                )
                .into());
            }
        }
        (m.disk_name(), if enabled { m.filename.clone() } else { format!("{}{DISABLED_SUFFIX}", m.filename) })
    };
    let dir = mods_dir(game)?;
    let (from, to) = (dir.join(&from), dir.join(&to));
    reject_link(&from)?;
    reject_link(&to)?;
    if !from.is_file() {
        return Err("Managed mod archive is missing on disk".into());
    }
    if to.exists() {
        return Err("Refusing to overwrite an existing mod archive".into());
    }
    fs::rename(&from, &to)?;
    reg.mods[idx].enabled = enabled;
    save(game, reg)
}

/// Remove a managed mod: delete its archive (active or disabled) and its
/// registry entry. Orphaned dependencies are NOT removed implicitly;
/// call `orphans` first and let the caller decide.
pub fn remove(game: &Path, reg: &mut Registry, project_id: &str) -> Result<ModEntry> {
    let idx = reg
        .mods
        .iter()
        .position(|m| m.project_id == project_id)
        .ok_or("Mod is not managed in this instance")?;
    if let Some(owner) = required_by_enabled(reg, project_id) {
        return Err(format!(
            "Cannot remove: project {owner} is enabled and requires this mod"
        )
        .into());
    }
    let entry = reg.mods.remove(idx);
    let path = mods_dir(game)?.join(entry.disk_name());
    reject_link(&path)?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    save(game, reg)?;
    Ok(entry)
}

/// Dependencies that were pulled in only for `project_id`.
pub fn orphans(reg: &Registry, project_id: &str) -> Vec<String> {
    reg.mods
        .iter()
        .filter(|m| m.required_by.as_deref() == Some(project_id))
        .map(|m| m.project_id.clone())
        .collect()
}

/// Archives in mods/ that Andromeda does not manage (user installs or
/// leftovers), active or disabled, excluding the registry itself.
pub fn untracked(game: &Path, reg: &Registry) -> Result<Vec<String>> {
    let dir = mods_dir(game)?;
    let managed: std::collections::BTreeSet<String> =
        reg.mods.iter().map(|m| m.disk_name()).collect();
    let mut out = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e.into()),
    };
    for e in entries {
        let name = e?.file_name().to_string_lossy().into_owned();
        if name == REGISTRY || name.starts_with('.') {
            continue;
        }
        let base = name.strip_suffix(DISABLED_SUFFIX).unwrap_or(&name);
        if (base.ends_with(".jar") || base.ends_with(".litemod")) && !managed.contains(&name) {
            out.push(name);
        }
    }
    out.sort();
    Ok(out)
}

/// Where a resolved file must land on disk.
pub fn dest_path(game: &Path, file: &ModFile) -> Result<PathBuf> {
    if !crate::modrinth::file_name_ok(&file.filename) {
        return Err("Unsafe Modrinth file name".into());
    }
    Ok(mods_dir(game)?.join(&file.filename))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_game(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "andromeda-mods-test-{}-{tag}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("mods")).unwrap();
        dir
    }

    fn entry(project: &str, file: &str, required_by: Option<&str>) -> ModEntry {
        ModEntry {
            project_id: project.into(),
            slug: project.to_lowercase(),
            title: project.into(),
            version_id: "VER0ver0".into(),
            version_number: "1.0".into(),
            filename: file.into(),
            sha512: "b".repeat(128),
            size: 10,
            mc: "1.12.2".into(),
            loader: "forge".into(),
            enabled: true,
            required_by: required_by.map(str::to_owned),
        }
    }

    #[test]
    fn missing_registry_loads_empty_and_roundtrips() {
        let game = test_game("roundtrip");
        let mut reg = load(&game).unwrap();
        assert!(reg.mods.is_empty());
        reg.mods.push(entry("AAAAaaaa", "a-1.0.jar", None));
        save(&game, &reg).unwrap();
        let back = load(&game).unwrap();
        assert_eq!(back.mods.len(), 1);
        assert_eq!(back.mods[0].filename, "a-1.0.jar");
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn enable_disable_renames_archive() {
        let game = test_game("toggle");
        let mods = game.join("mods");
        fs::write(mods.join("a-1.0.jar"), b"jar").unwrap();
        let mut reg = Registry {
            mods: vec![entry("AAAAaaaa", "a-1.0.jar", None)],
        };
        save(&game, &reg).unwrap();
        set_enabled(&game, &mut reg, "AAAAaaaa", false).unwrap();
        assert!(!mods.join("a-1.0.jar").exists());
        assert!(mods.join("a-1.0.jar.disabled").is_file());
        set_enabled(&game, &mut reg, "AAAAaaaa", true).unwrap();
        assert!(mods.join("a-1.0.jar").is_file());
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn remove_deletes_archive_and_entry() {
        let game = test_game("remove");
        let mods = game.join("mods");
        fs::write(mods.join("a-1.0.jar"), b"jar").unwrap();
        let mut reg = Registry {
            mods: vec![
                entry("AAAAaaaa", "a-1.0.jar", None),
                entry("LIB1lib1", "lib-2.0.jar", Some("AAAAaaaa")),
            ],
        };
        save(&game, &reg).unwrap();
        assert_eq!(orphans(&reg, "AAAAaaaa"), ["LIB1lib1".to_string()]);
        let gone = remove(&game, &mut reg, "AAAAaaaa").unwrap();
        assert_eq!(gone.project_id, "AAAAaaaa");
        assert!(!mods.join("a-1.0.jar").exists());
        assert!(reg.get("AAAAaaaa").is_none());
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn untracked_lists_manual_files_only() {
        let game = test_game("untracked");
        let mods = game.join("mods");
        fs::write(mods.join("a-1.0.jar"), b"jar").unwrap();
        fs::write(mods.join("manual.jar"), b"jar").unwrap();
        fs::write(mods.join("manual2.litemod.disabled"), b"mod").unwrap();
        fs::write(mods.join("readme.txt"), b"hi").unwrap();
        let reg = Registry {
            mods: vec![entry("AAAAaaaa", "a-1.0.jar", None)],
        };
        save(&game, &reg).unwrap();
        let reg = load(&game).unwrap();
        let extra = untracked(&game, &reg).unwrap();
        assert_eq!(extra, ["manual.jar", "manual2.litemod.disabled"]);
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn load_rejects_traversal_and_bad_digest() {
        let game = test_game("reject");
        let mut bad = entry("AAAAaaaa", "../evil.jar", None);
        let reg = Registry { mods: vec![bad.clone()] };
        assert!(save(&game, &reg).is_err());
        bad.filename = "ok.jar".into();
        bad.sha512 = "zz".into();
        let reg = Registry { mods: vec![bad] };
        assert!(save(&game, &reg).is_err());
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn disable_refuses_to_clobber_existing_target() {
        let game = test_game("clobber");
        let mods = game.join("mods");
        fs::write(mods.join("a-1.0.jar"), b"jar").unwrap();
        fs::write(mods.join("a-1.0.jar.disabled"), b"other").unwrap();
        let mut reg = Registry {
            mods: vec![entry("AAAAaaaa", "a-1.0.jar", None)],
        };
        save(&game, &reg).unwrap();
        assert!(set_enabled(&game, &mut reg, "AAAAaaaa", false).is_err());
        assert!(reg.mods[0].enabled, "failed toggle must not flip state");
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn required_mod_cannot_be_disabled_or_removed_while_dependent_enabled() {
        let game = test_game("guard");
        let mods = game.join("mods");
        fs::write(mods.join("a-1.0.jar"), b"jar").unwrap();
        fs::write(mods.join("lib-2.0.jar"), b"jar").unwrap();
        let mut reg = Registry {
            mods: vec![
                entry("AAAAaaaa", "a-1.0.jar", None),
                entry("LIB1lib1", "lib-2.0.jar", Some("AAAAaaaa")),
            ],
        };
        save(&game, &reg).unwrap();
        assert!(set_enabled(&game, &mut reg, "LIB1lib1", false).is_err());
        assert!(remove(&game, &mut reg, "LIB1lib1").is_err());
        assert!(mods.join("lib-2.0.jar").is_file(), "guarded mod must stay");
        // Once the dependent is disabled, the dependency may be touched.
        set_enabled(&game, &mut reg, "AAAAaaaa", false).unwrap();
        set_enabled(&game, &mut reg, "LIB1lib1", false).unwrap();
        assert!(mods.join("lib-2.0.jar.disabled").is_file());
        let _ = fs::remove_dir_all(&game);
    }
}
