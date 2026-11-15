//! Pinned upstream Forge/NeoForge client installers in separate launcher-owned roots.
//! Never points an installer at the user's .minecraft or an instance/game folder.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Forge,
    NeoForge,
}
impl Kind {
    pub fn version(self) -> &'static str {
        match self {
            Self::Forge => "61.2.5",
            Self::NeoForge => "21.11.48",
        }
    }
    fn id(self) -> &'static str {
        match self {
            Self::Forge => "1.21.11-forge-61.2.5",
            Self::NeoForge => "neoforge-21.11.48",
        }
    }
    fn main(self) -> &'static str {
        match self {
            Self::Forge => "net.minecraftforge.bootstrap.ForgeBootstrap",
            Self::NeoForge => "net.neoforged.fml.startup.Client",
        }
    }
    fn url(self) -> &'static str {
        match self {Self::Forge=>"https://maven.minecraftforge.net/net/minecraftforge/forge/1.21.11-61.2.5/forge-1.21.11-61.2.5-installer.jar",Self::NeoForge=>"https://maven.neoforged.net/releases/net/neoforged/neoforge/21.11.48/neoforge-21.11.48-installer.jar"}
    }
    fn hash(self) -> &'static str {
        match self {
            Self::Forge => "4b1a7d08cd598f7e2513c10c239acc581b3934a455ea9752a1ef43de02914221",
            Self::NeoForge => "1f14a4bac4aa885a497550fa5a8374451b7014cec27e1bb69ab16742fb689db0",
        }
    }
}
pub struct Installed {
    pub profile: Value,
    pub libraries: Vec<PathBuf>,
    pub library_dir: PathBuf,
}
fn plain(p: &Path) -> Result<()> {
    let m = fs::symlink_metadata(p)?;
    if m.file_type().is_symlink() {
        return Err("Installer link rejected".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if m.file_attributes() & 0x400 != 0 {
            return Err("Installer reparse point rejected".into());
        }
    }
    if !m.is_file() && !m.is_dir() {
        return Err("Installer special file rejected".into());
    }
    Ok(())
}
fn tree(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    plain(root)?;
    for e in fs::read_dir(root)? {
        let p = e?.path();
        plain(&p)?;
        if p.is_dir() {
            tree(&p, files)?;
        } else {
            files.push(p);
        }
    }
    Ok(())
}
fn digest(p: &Path) -> Result<String> {
    plain(p)?;
    let mut f = fs::File::open(p)?;
    let mut h = Sha256::new();
    let mut b = [0; 65536];
    loop {
        let n = f.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
fn publish(p: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let temp = p.with_extension(format!("{}.part", std::process::id()));
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let result = (|| -> std::io::Result<()> {
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        fs::rename(&temp, p)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result?;
    Ok(())
}
fn profile(jar: &Path, kind: Kind) -> Result<Value> {
    let mut zip = zip::ZipArchive::new(fs::File::open(jar)?)?;
    let f = zip.by_name("version.json")?;
    if f.size() > 2 * 1024 * 1024 {
        return Err("Installer profile too large".into());
    }
    let p: Value = serde_json::from_reader(f)?;
    validate_profile(&p, kind)?;
    Ok(p)
}
fn validate_profile(p: &Value, kind: Kind) -> Result<()> {
    if p["id"] != kind.id() || p["inheritsFrom"] != "1.21.11" || p["mainClass"] != kind.main() {
        return Err("Installer profile does not match the selected pinned loader".into());
    }
    let libs = p["libraries"]
        .as_array()
        .ok_or("Missing installer libraries")?;
    if libs.is_empty() || libs.len() > 250 {
        return Err("Invalid installer library count".into());
    }
    let mut paths = std::collections::HashMap::new();
    for lib in libs {
        let a = &lib["downloads"]["artifact"];
        let rel = a["path"].as_str().ok_or("Missing installer library path")?;
        crate::vanilla::path(Path::new("cache"), rel)?;
        let hash = a["sha1"]
            .as_str()
            .ok_or("Missing installer library digest")?;
        if hash.len() != 40
            || !hash.bytes().all(|b| b.is_ascii_hexdigit())
            || a["size"].as_u64().is_none_or(|s| s == 0)
        {
            return Err("Invalid/duplicate installer library".into());
        }
        if let Some(previous) = paths.insert(rel, (hash, a["size"].as_u64())) {
            if previous != (hash, a["size"].as_u64()) {
                return Err("Conflicting duplicate installer library".into());
            }
        }
    }
    Ok(())
}
/// `java` and `vanilla` must have been checked by the vanilla launch-plan builder.
pub fn install(root: &Path, java: &Path, vanilla: &Path, kind: Kind) -> Result<Installed> {
    let root = std::path::absolute(root)?;
    fs::create_dir_all(&root)?;
    let mut existing = Vec::new();
    tree(&root, &mut existing)?;
    let jar = root.join("installer.jar");
    if !jar.exists() {
        let c = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(120))
            .build()?;
        let r = c.get(kind.url()).send()?.error_for_status()?;
        if r.content_length().is_some_and(|n| n > 64 * 1024 * 1024) {
            return Err("Installer too large".into());
        }
        let mut bytes = Vec::new();
        r.take(64 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > 64 * 1024 * 1024 || format!("{:x}", Sha256::digest(&bytes)) != kind.hash()
        {
            return Err("Installer digest mismatch".into());
        }
        publish(&jar, &bytes)?;
    }
    if digest(&jar)? != kind.hash() {
        return Err("Existing installer digest mismatch; not overwritten".into());
    }
    let profile = profile(&jar, kind)?;
    let receipt = root.join("verified-install.json");
    let libraries = root.join("libraries");
    if !receipt.exists() {
        let versions = root.join("versions/1.21.11");
        fs::create_dir_all(&versions)?;
        // Only copy the selected, verified vanilla version. No account or launcher settings are copied.
        for ext in ["jar", "json"] {
            let dst = versions.join(format!("1.21.11.{ext}"));
            if dst.exists() {
                plain(&dst)?;
            }
            fs::copy(vanilla.join(format!("versions/1.21.11/1.21.11.{ext}")), dst)?;
        }
        let lp = root.join("launcher_profiles.json");
        if !lp.exists() {
            publish(&lp, b"{\"profiles\":{}}")?;
        }
        let out = fs::File::create(root.join("installer-output.log"))?;
        let err = out.try_clone()?;
        let mut cmd = Command::new(java);
        cmd.arg("-jar")
            .arg(&jar)
            .arg("--installClient")
            .arg(&root)
            .current_dir(&root)
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let mut child = cmd.spawn()?;
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                if !status.success() {
                    return Err(
                        "Upstream client installer failed. See dedicated installer-output.log"
                            .into(),
                    );
                }
                break;
            }
            if start.elapsed() > Duration::from_secs(600) {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Upstream client installer timed out".into());
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        // A successful installer exit is not enough: check every launch library against the embedded official profile.
        let mut files = Vec::new();
        tree(&libraries, &mut files)?;
        for lib in profile["libraries"].as_array().ok_or("Missing libraries")? {
            let a = &lib["downloads"]["artifact"];
            let p = crate::vanilla::path(&libraries, a["path"].as_str().ok_or("Missing path")?)?;
            if !crate::vanilla::valid(
                &p,
                a["sha1"].as_str().ok_or("Missing hash")?,
                a["size"].as_u64().ok_or("Missing size")?,
            )? {
                return Err("Installed launch library failed official verification".into());
            }
        }
        let mut checks = Vec::new();
        for p in files {
            checks.push(json!({"path":p.strip_prefix(&root)?.to_string_lossy().replace('\\',"/"),"size":fs::metadata(&p)?.len(),"sha256":digest(&p)?}));
        }
        publish(
            &receipt,
            &serde_json::to_vec(
                &json!({"id":kind.id(),"installer_sha256":kind.hash(),"files":checks}),
            )?,
        )?;
    }
    plain(&receipt)?;
    if fs::metadata(&receipt)?.len() > 4 * 1024 * 1024 {
        return Err("Installer receipt too large".into());
    }
    let checks: Value = serde_json::from_slice(&fs::read(&receipt)?)?;
    if checks["id"] != kind.id() || checks["installer_sha256"] != kind.hash() {
        return Err("Installer receipt mismatch".into());
    }
    let files = checks["files"]
        .as_array()
        .ok_or("Missing installed library checks")?;
    if files.is_empty() || files.len() > 1000 {
        return Err("Invalid installed file count".into());
    }
    for f in files {
        let p = crate::vanilla::path(&root, f["path"].as_str().ok_or("Missing installed path")?)?;
        if fs::metadata(&p)?.len() != f["size"].as_u64().ok_or("Missing installed size")?
            || digest(&p)? != f["sha256"].as_str().ok_or("Missing installed digest")?
        {
            return Err("Installed loader file changed; not overwritten".into());
        }
    }
    // Recheck every official launch artifact on cached runs as well. A receipt cannot omit one.
    for lib in profile["libraries"]
        .as_array()
        .ok_or("Missing launch libraries")?
    {
        let a = &lib["downloads"]["artifact"];
        let p = crate::vanilla::path(&libraries, a["path"].as_str().ok_or("Missing launch path")?)?;
        plain(&p)?;
        if !crate::vanilla::valid(
            &p,
            a["sha1"].as_str().ok_or("Missing hash")?,
            a["size"].as_u64().ok_or("Missing size")?,
        )? {
            return Err("Cached launch library failed official verification".into());
        }
    }
    if kind == Kind::NeoForge {
        for required in ["libraries/net/neoforged/minecraft-client-patched/21.11.48/minecraft-client-patched-21.11.48.jar","libraries/net/neoforged/neoforge/21.11.48/neoforge-21.11.48-universal.jar"] {
   if !files.iter().any(|f|f["path"].as_str()==Some(required)){return Err("Required generated NeoForge file absent from receipt".into());}
  }
    }
    let mut launch_libraries = Vec::new();
    for lib in profile["libraries"]
        .as_array()
        .ok_or("Missing launch libraries")?
    {
        launch_libraries.push(crate::vanilla::path(
            &libraries,
            lib["downloads"]["artifact"]["path"]
                .as_str()
                .ok_or("Missing launch path")?,
        )?);
    }
    Ok(Installed {
        profile,
        libraries: launch_libraries,
        library_dir: libraries,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn p() -> Value {
        json!({"id":"1.21.11-forge-61.2.5","inheritsFrom":"1.21.11","mainClass":"net.minecraftforge.bootstrap.ForgeBootstrap","libraries":[{"downloads":{"artifact":{"path":"a/b.jar","sha1":"0123456789012345678901234567890123456789","size":1}}}]})
    }
    #[test]
    fn pinned_profile() {
        assert!(validate_profile(&p(), Kind::Forge).is_ok());
        assert!(validate_profile(&p(), Kind::NeoForge).is_err());
    }
    #[test]
    fn traversal() {
        let mut p = p();
        p["libraries"][0]["downloads"]["artifact"]["path"] = json!("../evil.jar");
        assert!(validate_profile(&p, Kind::Forge).is_err());
    }
    #[test]
    fn wrong_game() {
        let mut p = p();
        p["inheritsFrom"] = json!("1.21.10");
        assert!(validate_profile(&p, Kind::Forge).is_err());
    }
}
