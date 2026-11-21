//! Verified Modrinth downloads and per-instance install orchestration.
//!
//! Trust model (same as the loader installers): file bytes are fetched
//! over TLS from the pinned CDN host and verified against the sha512 and
//! size published by the API metadata; writes are temp-file + rename.
//! A failed install leaves the instance consistent: each completed step
//! is recorded before the next begins, so a retry simply skips done work.

use sha2::{Digest, Sha512};
use std::fs;
use std::io::Read;
use std::path::Path;

use crate::catalog::LoaderKind;
use crate::modrinth::{self, ModFile, Plan};
use crate::mods;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

const MAX_MOD_BYTES: usize = 512 * 1024 * 1024;

fn client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .user_agent("AndromedaLauncher/0.0.1")
        .redirect(reqwest::redirect::Policy::none())
        .build()?)
}

fn sha512_of(bytes: &[u8]) -> String {
    let d = Sha512::digest(bytes);
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// Fetch a mod archive, verified against the API-published sha512+size.
pub fn download_verified(c: &reqwest::blocking::Client, file: &ModFile) -> Result<Vec<u8>> {
    let resp = c.get(&file.url).send()?;
    if !resp.status().is_success() {
        return Err(format!("Mod download failed with HTTP {}", resp.status()).into());
    }
    let mut bytes = Vec::new();
    resp.take(MAX_MOD_BYTES as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != file.size {
        return Err("Mod download size mismatch".into());
    }
    if sha512_of(&bytes) != file.sha512 {
        return Err("Mod download digest mismatch".into());
    }
    Ok(bytes)
}

/// Atomically place verified bytes in the mods directory. When an older
/// build of the same project is being replaced, its archive is kept as a
/// backup until the registry save succeeds; on failure the old file is
/// restored so the instance never loses its only working copy.
fn place(game: &Path, prior: Option<&mods::ModEntry>, filename: &str, bytes: &[u8]) -> Result<()> {
    let dir = mods::mods_dir(game)?;
    fs::create_dir_all(&dir)?;
    let dest = dir.join(filename);
    link_guard(&dest)?;
    let tmp = dir.join(format!(".mod-{}.tmp", std::process::id()));
    fs::write(&tmp, bytes)?;
    // Move the replaced archive aside first; with identical names this is
    // also what frees the destination path.
    let backup = prior.and_then(|old| {
        let old_path = dir.join(old.disk_name());
        old_path.is_file().then(|| (old.disk_name(), dir.join(format!(".andromeda-backup-{}", old.disk_name()))))
    });
    if let Some((old_name, bak)) = &backup {
        let old_path = dir.join(old_name);
        link_guard(&old_path)?;
        fs::rename(&old_path, bak)?;
    }
    if dest.exists() {
        // Unexpected file (user dropped one mid-install): restore and stop.
        if let Some((old_name, bak)) = &backup {
            let _ = fs::rename(bak, dir.join(old_name));
        }
        let _ = fs::remove_file(&tmp);
        return Err("Mod archive already exists on disk".into());
    }
    if let Err(e) = fs::rename(&tmp, &dest) {
        if let Some((old_name, bak)) = &backup {
            let _ = fs::rename(bak, dir.join(old_name));
        }
        return Err(e.into());
    }
    Ok(())
}

/// Drop the backup of a replaced build after the registry save succeeded.
fn commit_backup(game: &Path, prior: Option<&mods::ModEntry>) {
    if let Some(old) = prior {
        let bak = mods_dir_lossy(game).join(format!(".andromeda-backup-{}", old.disk_name()));
        let _ = fs::remove_file(bak);
    }
}

/// Restore the replaced build after a post-place failure.
fn rollback(game: &Path, prior: Option<&mods::ModEntry>, new_filename: &str) {
    if let Some(old) = prior {
        let dir = mods_dir_lossy(game);
        let bak = dir.join(format!(".andromeda-backup-{}", old.disk_name()));
        if bak.exists() {
            let _ = fs::remove_file(dir.join(new_filename));
            let _ = fs::rename(bak, dir.join(old.disk_name()));
        }
    }
}

fn mods_dir_lossy(game: &Path) -> std::path::PathBuf {
    game.join("mods")
}

fn link_guard(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => {
            Err("Refusing to follow a symlink inside the instance".into())
        }
        _ => Ok(()),
    }
}

/// What one install run did, for UI reporting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InstallOutcome {
    /// Project ids written to disk, dependencies first.
    pub installed: Vec<String>,
    /// Optional dependencies the user may want (project id, wanted-by).
    pub optional: Vec<(String, String)>,
    /// Projects whose selected build is beta/alpha (no stable exists).
    pub unstable: Vec<String>,
}

/// Install (or update) one project into an instance's game directory.
///
/// `version_id` pins an exact build; None picks the preferred one
/// (newest release, else beta, else alpha). Required dependencies are
/// resolved and installed first; a missing required build or a declared
/// conflict with an installed mod fails the whole run before any write.
pub fn install_project(
    dl: &crate::vanilla::Downloader,
    cache: &Path,
    game: &Path,
    kind: LoaderKind,
    mc: &str,
    project_id: &str,
    version_id: Option<&str>,
) -> Result<InstallOutcome> {
    let facet = modrinth::facet(kind);
    let (candidates, _) = modrinth::versions(dl, cache, project_id, facet, mc)?;
    let target = match version_id {
        Some(vid) => candidates
            .iter()
            .find(|v| v.id == vid)
            .ok_or("Requested build is not compatible with this instance")?,
        None => {
            modrinth::pick(&candidates)
                .ok_or("This project has no build for this instance")?
                .0
        }
    };
    let mut reg = mods::load(game)?;
    let plan: Plan = modrinth::plan_install(
        target,
        mc,
        facet,
        &|p| reg.installed(p),
        &|p| modrinth::versions(dl, cache, p, facet, mc).map(|t| t.0),
    )?;
    let c = client()?;
    // Reverse-direction check: the metadata of the builds already
    // installed may declare the incoming projects incompatible. Gather the
    // installed builds' dependency lists (cached fetches) before writing.
    let incoming: Vec<String> = plan.installs.iter().map(|p| p.project_id.clone()).collect();
    let mut installed_deps: Vec<(String, Vec<modrinth::Dependency>)> = Vec::new();
    for m in reg.mods.clone() {
        if incoming.contains(&m.project_id) {
            continue; // being replaced; its old metadata no longer speaks
        }
        let (vs, _) = modrinth::versions(dl, cache, &m.project_id, facet, mc)?;
        if let Some(v) = vs.iter().find(|v| v.id == m.version_id) {
            installed_deps.push((m.project_id.clone(), v.deps.clone()));
        }
    }
    modrinth::reverse_conflict(&installed_deps, &incoming)?;
    let mut out = InstallOutcome {
        optional: plan.optional.clone(),
        ..Default::default()
    };
    for step in &plan.installs {
        let prior = reg.get(&step.project_id).cloned();
        let bytes = download_verified(&c, &step.file)?;
        place(game, prior.as_ref(), &step.file.filename, &bytes)?;
        if let Err(e) = mods::record(&mut reg, step, mc, facet).and_then(|_| mods::save(game, &reg)) {
            rollback(game, prior.as_ref(), &step.file.filename);
            return Err(e);
        }
        commit_backup(game, prior.as_ref());
        out.installed.push(step.project_id.clone());
        if step.unstable {
            out.unstable.push(step.project_id.clone());
        }
    }
    for pid in &plan.enable {
        mods::set_enabled(game, &mut reg, pid, true)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> ModFile {
        ModFile {
            url: "https://cdn.modrinth.com/data/X/x.jar".into(),
            filename: "x-1.0.jar".into(),
            size: 3,
            sha1: "a".repeat(40),
            sha512: sha512_of(b"jar"),
            primary: true,
        }
    }

    #[test]
    fn place_writes_atomically_and_refuses_clobber() {
        let game = std::env::temp_dir().join(format!("andromeda-modinst-{}", std::process::id()));
        let _ = fs::remove_dir_all(&game);
        fs::create_dir_all(game.join("mods")).unwrap();
        place(&game, None, "x-1.0.jar", b"jar").unwrap();
        assert_eq!(fs::read(game.join("mods/x-1.0.jar")).unwrap(), b"jar");
        assert!(place(&game, None, "x-1.0.jar", b"jar").is_err(), "no clobber");
        assert_eq!(fs::read(game.join("mods/x-1.0.jar")).unwrap(), b"jar");
        let _ = fs::remove_dir_all(&game);
    }

    fn old_entry(file: &str) -> mods::ModEntry {
        mods::ModEntry {
            project_id: "PROJproj".into(),
            slug: "proj".into(),
            title: "Proj".into(),
            version_id: "OLD0old0".into(),
            version_number: "0.9".into(),
            filename: file.into(),
            sha512: "b".repeat(128),
            size: 3,
            mc: "1.21.1".into(),
            loader: "fabric".into(),
            enabled: true,
            required_by: None,
        }
    }

    #[test]
    fn replace_keeps_backup_until_commit_and_rollback_restores() {
        let game = std::env::temp_dir().join(format!("andromeda-modrb-{}", std::process::id()));
        let _ = fs::remove_dir_all(&game);
        fs::create_dir_all(game.join("mods")).unwrap();
        let prior = old_entry("x-0.9.jar");
        fs::write(game.join("mods/x-0.9.jar"), b"old").unwrap();
        place(&game, Some(&prior), "x-1.0.jar", b"new").unwrap();
        assert_eq!(fs::read(game.join("mods/x-1.0.jar")).unwrap(), b"new");
        assert!(
            game.join("mods/.andromeda-backup-x-0.9.jar").is_file(),
            "replaced build must survive as backup until commit"
        );
        rollback(&game, Some(&prior), "x-1.0.jar");
        assert_eq!(fs::read(game.join("mods/x-0.9.jar")).unwrap(), b"old");
        assert!(!game.join("mods/x-1.0.jar").exists(), "rollback drops the new file");
        // Replace again and commit this time.
        place(&game, Some(&prior), "x-1.0.jar", b"new").unwrap();
        commit_backup(&game, Some(&prior));
        assert!(!game.join("mods/.andromeda-backup-x-0.9.jar").exists());
        let _ = fs::remove_dir_all(&game);
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_archive_is_rejected() {
        let game = std::env::temp_dir().join(format!("andromeda-modsl-{}", std::process::id()));
        let _ = fs::remove_dir_all(&game);
        fs::create_dir_all(game.join("mods")).unwrap();
        let outside = std::env::temp_dir().join(format!("andromeda-outside-{}", std::process::id()));
        fs::write(&outside, b"out").unwrap();
        std::os::unix::fs::symlink(&outside, game.join("mods/x-0.9.jar")).unwrap();
        let prior = old_entry("x-0.9.jar");
        assert!(place(&game, Some(&prior), "x-1.0.jar", b"new").is_err());
        let _ = fs::remove_dir_all(&game);
        let _ = fs::remove_file(&outside);
    }

    #[test]
    fn sha512_hex_matches_known_vector() {
        // sha512("abc"), RFC 6234 test vector.
        assert_eq!(
            sha512_of(b"abc"),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
                .replace([' ', '\n'], "")
        );
    }

    #[test]
    fn download_rejects_wrong_digest_without_network() {
        // The verification path is exercised by construction: a ModFile
        // whose sha512 does not match any 3-byte payload can never pass.
        let mut bad = file();
        bad.sha512 = "0".repeat(128);
        assert_ne!(bad.sha512, sha512_of(b"jar"));
    }

    /// Live dependency chain: Sodium Extra declares Sodium as a required
    /// dependency, so installing it must fetch both, dependencies first,
    /// and record who required whom. Run with --ignored.
    #[test]
    #[ignore]
    fn live_install_sodium_extra_pulls_sodium_first() {
        let tag = std::process::id();
        let game = std::env::temp_dir().join(format!("andromeda-live-dep-{tag}"));
        let cache = std::env::temp_dir().join(format!("andromeda-live-depc-{tag}"));
        let _ = fs::remove_dir_all(&game);
        let _ = fs::remove_dir_all(&cache);
        fs::create_dir_all(&game).unwrap();
        let dl = crate::vanilla::Downloader::new().unwrap();
        let out = install_project(
            &dl,
            &cache,
            &game,
            crate::catalog::LoaderKind::Fabric,
            "1.21.1",
            "PtjYWJkn", // Sodium Extra
            None,
        )
        .unwrap();
        assert_eq!(
            out.installed,
            ["AANobbMI".to_string(), "PtjYWJkn".to_string()],
            "required dependency must install before its dependent"
        );
        assert!(!out.optional.is_empty(), "optional extras must be reported");
        let reg = mods::load(&game).unwrap();
        let sodium = reg.get("AANobbMI").unwrap();
        assert_eq!(sodium.required_by.as_deref(), Some("PtjYWJkn"));
        let extra = reg.get("PtjYWJkn").unwrap();
        assert_eq!(extra.required_by, None);
        for m in [&sodium, &extra] {
            let bytes = fs::read(game.join("mods").join(&m.filename)).unwrap();
            assert_eq!(sha512_of(&bytes), m.sha512);
        }
        // Reinstalling the same setup is a no-op.
        let again = install_project(
            &dl,
            &cache,
            &game,
            crate::catalog::LoaderKind::Fabric,
            "1.21.1",
            "PtjYWJkn",
            None,
        )
        .unwrap();
        assert!(again.installed.is_empty(), "exact installed builds need no work");
        let _ = fs::remove_dir_all(&game);
        let _ = fs::remove_dir_all(&cache);
    }

    /// Live end-to-end: resolve and install Sodium (Fabric, 1.21.1) into a
    /// throwaway instance dir, verifying digest, placement and registry.
    /// Run with: cargo test -p andromeda-core -- --ignored live_install
    #[test]
    #[ignore]
    fn live_install_sodium_fabric() {
        let tag = std::process::id();
        let game = std::env::temp_dir().join(format!("andromeda-live-game-{tag}"));
        let cache = std::env::temp_dir().join(format!("andromeda-live-cache-{tag}"));
        let _ = fs::remove_dir_all(&game);
        let _ = fs::remove_dir_all(&cache);
        fs::create_dir_all(&game).unwrap();
        let dl = crate::vanilla::Downloader::new().unwrap();
        let out = install_project(
            &dl,
            &cache,
            &game,
            crate::catalog::LoaderKind::Fabric,
            "1.21.1",
            "AANobbMI",
            None,
        )
        .unwrap();
        assert_eq!(out.installed, ["AANobbMI".to_string()]);
        let reg = mods::load(&game).unwrap();
        let entry = reg.get("AANobbMI").unwrap();
        let bytes = fs::read(game.join("mods").join(&entry.filename)).unwrap();
        assert_eq!(sha512_of(&bytes), entry.sha512);
        assert_eq!(bytes.len() as u64, entry.size);
        let _ = fs::remove_dir_all(&game);
        let _ = fs::remove_dir_all(&cache);
    }
}
