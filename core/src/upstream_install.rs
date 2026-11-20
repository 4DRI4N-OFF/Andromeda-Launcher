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
    fn hash(self) -> &'static str {
        match self {
            Self::Forge => "4b1a7d08cd598f7e2513c10c239acc581b3934a455ea9752a1ef43de02914221",
            Self::NeoForge => "1f14a4bac4aa885a497550fa5a8374451b7014cec27e1bb69ab16742fb689db0",
        }
    }
}

/// Loader entry points observed in real installer profiles across
/// generations (Forge 1.12.2 through 1.21.11, NeoForge 1.20.1 through
/// 1.21.11; surveyed 2026-11-20). A profile whose main class is not in this
/// list is rejected.
const MAIN_CLASSES: [&str; 5] = [
    "net.minecraftforge.bootstrap.ForgeBootstrap",
    "cpw.mods.bootstraplauncher.BootstrapLauncher",
    "cpw.mods.modlauncher.Launcher",
    "net.minecraft.launchwrapper.Launch",
    "net.neoforged.fml.startup.Client",
];

/// A specific installer published upstream: loader kind, Minecraft version
/// and bare loader build ("61.2.5", "21.11.48", "47.1.106").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spec {
    kind: Kind,
    mc: String,
    version: String,
}
impl Spec {
    pub fn new(kind: Kind, mc: &str, version: &str) -> Result<Self> {
        crate::instances::validate_mc_id(mc)?;
        crate::instances::validate_loader_version(version)?;
        Ok(Self {
            kind,
            mc: mc.to_owned(),
            version: version.to_owned(),
        })
    }
    /// The build this launcher originally pinned, kept as a known-good spec.
    pub fn pinned(kind: Kind) -> Self {
        Self {
            kind,
            mc: "1.21.11".into(),
            version: kind.version().into(),
        }
    }
    pub fn kind(&self) -> Kind {
        self.kind
    }
    pub fn mc(&self) -> &str {
        &self.mc
    }
    pub fn version(&self) -> &str {
        &self.version
    }
    fn is_pinned(&self) -> bool {
        self.mc == "1.21.11" && self.version == self.kind.version()
    }
    /// NeoForge for Minecraft 1.20.1 is published in a separate artifact
    /// that follows the Forge layout (combined "1.20.1-47.1.106").
    fn neoforge_legacy_artifact(&self) -> bool {
        self.kind == Kind::NeoForge && self.version.starts_with("47.1.")
    }
    /// Directory/artifact version component used upstream.
    fn combined(&self) -> String {
        match self.kind {
            Kind::Forge => format!("{}-{}", self.mc, self.version),
            Kind::NeoForge if self.neoforge_legacy_artifact() => {
                format!("{}-{}", self.mc, self.version)
            }
            Kind::NeoForge => self.version.clone(),
        }
    }
    /// Maven artifact directory: forge or neoforge.
    fn artifact(&self) -> &'static str {
        match self.kind {
            Kind::Forge => "forge",
            Kind::NeoForge if self.neoforge_legacy_artifact() => "forge",
            Kind::NeoForge => "neoforge",
        }
    }
    pub fn installer_url(&self) -> String {
        let (c, a) = (self.combined(), self.artifact());
        match self.kind {
            Kind::Forge => format!(
                "https://maven.minecraftforge.net/net/minecraftforge/forge/{c}/forge-{c}-installer.jar"
            ),
            Kind::NeoForge => format!(
                "https://maven.neoforged.net/releases/net/neoforged/{a}/{c}/{a}-{c}-installer.jar"
            ),
        }
    }
    /// Version id the installer profile must declare.
    pub fn id(&self) -> String {
        match self.kind {
            Kind::Forge => format!("{}-forge-{}", self.mc, self.version),
            // Verified 2026-11-20: the 1.20.1 NeoForge profile declares
            // id "1.20.1-forge-47.1.106", following the Forge pattern.
            Kind::NeoForge if self.neoforge_legacy_artifact() => {
                format!("{}-forge-{}", self.mc, self.version)
            }
            Kind::NeoForge => format!("neoforge-{}", self.version),
        }
    }
    /// Digest embedded in this launcher, known only for the pinned builds.
    fn embedded_hash(&self) -> Option<&'static str> {
        self.is_pinned().then(|| self.kind.hash())
    }
}
/// Extract the digest from a Maven `.sha256` sidecar: the first
/// whitespace-separated token, 64 lowercase hex digits.
fn parse_sha256_sidecar(body: &str) -> Result<String> {
    let token = body.split_whitespace().next().ok_or("Empty sidecar")?;
    let token = token.to_ascii_lowercase();
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid sidecar digest".into());
    }
    Ok(token)
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
fn profile(jar: &Path, spec: &Spec) -> Result<Value> {
    let mut zip = zip::ZipArchive::new(fs::File::open(jar)?)?;
    let f = zip.by_name("version.json")?;
    if f.size() > 2 * 1024 * 1024 {
        return Err("Installer profile too large".into());
    }
    let p: Value = serde_json::from_reader(f)?;
    validate_profile(&p, spec)?;
    Ok(p)
}
fn validate_profile(p: &Value, spec: &Spec) -> Result<()> {
    let main = p["mainClass"].as_str().unwrap_or("");
    if p["id"] != spec.id() || p["inheritsFrom"] != spec.mc() || !MAIN_CLASSES.contains(&main) {
        return Err("Installer profile does not match the selected loader".into());
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
    install_spec(root, java, vanilla, &Spec::pinned(kind))
}
/// Install any published upstream build. The installer digest comes from the
/// Maven `.sha256` sidecar (cached next to the installer); pinned builds must
/// additionally match the digest embedded in this launcher.
pub fn install_spec(root: &Path, java: &Path, vanilla: &Path, spec: &Spec) -> Result<Installed> {
    let root = std::path::absolute(root)?;
    fs::create_dir_all(&root)?;
    let mut existing = Vec::new();
    tree(&root, &mut existing)?;
    let jar = root.join("installer.jar");
    let sidecar = root.join("installer.sha256");
    let expected = match fs::read_to_string(&sidecar)
        .ok()
        .and_then(|body| parse_sha256_sidecar(&body).ok())
    {
        Some(hash) => hash,
        None => {
            let body = crate::vanilla::Downloader::new()?
                .text(&format!("{}.sha256", spec.installer_url()))?;
            let hash = parse_sha256_sidecar(&body)?;
            if let Some(embedded) = spec.embedded_hash() {
                if hash != embedded {
                    return Err("Sidecar digest does not match the pinned digest".into());
                }
            }
            if !sidecar.exists() {
                publish(&sidecar, hash.as_bytes())?;
            }
            hash
        }
    };
    if !jar.exists() {
        let c = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(120))
            .build()?;
        let r = c.get(spec.installer_url()).send()?.error_for_status()?;
        if r.content_length().is_some_and(|n| n > 64 * 1024 * 1024) {
            return Err("Installer too large".into());
        }
        let mut bytes = Vec::new();
        r.take(64 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > 64 * 1024 * 1024 || format!("{:x}", Sha256::digest(&bytes)) != expected {
            return Err("Installer digest mismatch".into());
        }
        publish(&jar, &bytes)?;
    }
    if digest(&jar)? != expected {
        return Err("Existing installer digest mismatch; not overwritten".into());
    }
    let profile = profile(&jar, spec)?;
    let receipt = root.join("verified-install.json");
    let libraries = root.join("libraries");
    if !receipt.exists() {
        let versions = root.join(format!("versions/{}", spec.mc()));
        fs::create_dir_all(&versions)?;
        // Only copy the selected, verified vanilla version. No account or launcher settings are copied.
        for ext in ["jar", "json"] {
            let dst = versions.join(format!("{}.{ext}", spec.mc()));
            if dst.exists() {
                plain(&dst)?;
            }
            fs::copy(
                vanilla.join(format!("versions/{0}/{0}.{ext}", spec.mc())),
                dst,
            )?;
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
                &json!({"id":spec.id(),"installer_sha256":expected,"files":checks}),
            )?,
        )?;
    }
    plain(&receipt)?;
    if fs::metadata(&receipt)?.len() > 4 * 1024 * 1024 {
        return Err("Installer receipt too large".into());
    }
    let checks: Value = serde_json::from_slice(&fs::read(&receipt)?)?;
    if checks["id"] != spec.id() || checks["installer_sha256"] != expected {
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
    if spec.kind() == Kind::NeoForge {
        // Files the installer must have generated, in the upstream layout of
        // this artifact (neoforge modern, forge for the 1.20.1 line).
        let (a, c) = (spec.artifact(), spec.combined());
        let universal = format!("libraries/net/neoforged/{a}/{c}/{a}-{c}-universal.jar");
        if !files.iter().any(|f| f["path"].as_str() == Some(&universal)) {
            return Err("Required generated NeoForge file absent from receipt".into());
        }
        // The patched client moved with the 21.4 generation: older lines
        // ship it as {artifact}-{version}-client.jar next to the universal,
        // newer ones as the separate minecraft-client-patched artifact. Both
        // layouts are published upstream; anything else is rejected.
        let patched_current = format!(
            "libraries/net/neoforged/minecraft-client-patched/{c}/minecraft-client-patched-{c}.jar"
        );
        let patched_legacy = format!("libraries/net/neoforged/{a}/{c}/{a}-{c}-client.jar");
        if !files.iter().any(|f| {
            f["path"].as_str() == Some(&patched_current)
                || f["path"].as_str() == Some(&patched_legacy)
        }) {
            return Err("Required generated NeoForge client absent from receipt".into());
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
        assert!(validate_profile(&p(), &Spec::pinned(Kind::Forge)).is_ok());
        assert!(validate_profile(&p(), &Spec::pinned(Kind::NeoForge)).is_err());
    }
    #[test]
    fn traversal() {
        let mut p = p();
        p["libraries"][0]["downloads"]["artifact"]["path"] = json!("../evil.jar");
        assert!(validate_profile(&p, &Spec::pinned(Kind::Forge)).is_err());
    }
    #[test]
    fn wrong_game() {
        let mut p = p();
        p["inheritsFrom"] = json!("1.21.10");
        assert!(validate_profile(&p, &Spec::pinned(Kind::Forge)).is_err());
    }
    #[test]
    fn spec_urls_cover_every_generation() {
        let forge = Spec::new(Kind::Forge, "1.20.1", "47.4.29").unwrap();
        assert_eq!(
            forge.installer_url(),
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.4.29/forge-1.20.1-47.4.29-installer.jar"
        );
        assert_eq!(forge.id(), "1.20.1-forge-47.4.29");
        let nf = Spec::new(Kind::NeoForge, "1.21.1", "21.1.267").unwrap();
        assert_eq!(
            nf.installer_url(),
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.267/neoforge-21.1.267-installer.jar"
        );
        assert_eq!(nf.id(), "neoforge-21.1.267");
        // The 1.20.1-era NeoForge artifact follows the Forge layout,
        // including the "-forge-" profile id (verified 2026-11-20).
        let nf1201 = Spec::new(Kind::NeoForge, "1.20.1", "47.1.106").unwrap();
        assert_eq!(
            nf1201.installer_url(),
            "https://maven.neoforged.net/releases/net/neoforged/forge/1.20.1-47.1.106/forge-1.20.1-47.1.106-installer.jar"
        );
        assert_eq!(nf1201.id(), "1.20.1-forge-47.1.106");
        assert!(Spec::new(Kind::Forge, "../etc", "47.4.29").is_err());
        assert!(Spec::new(Kind::Forge, "1.20.1", "..\\x").is_err());
    }
    #[test]
    fn every_generation_main_class_is_accepted() {
        for (mc, version, main) in [
            (
                "1.12.2",
                "14.23.5.2864",
                "net.minecraft.launchwrapper.Launch",
            ),
            ("1.16.5", "36.2.42", "cpw.mods.modlauncher.Launcher"),
            (
                "1.20.1",
                "47.4.29",
                "cpw.mods.bootstraplauncher.BootstrapLauncher",
            ),
            (
                "1.21.11",
                "61.2.5",
                "net.minecraftforge.bootstrap.ForgeBootstrap",
            ),
        ] {
            let spec = Spec::new(Kind::Forge, mc, version).unwrap();
            let mut p = p();
            p["id"] = json!(spec.id());
            p["inheritsFrom"] = json!(mc);
            p["mainClass"] = json!(main);
            assert!(validate_profile(&p, &spec).is_ok(), "{mc} {version}");
        }
        // An unknown entry point stays rejected even with a matching id.
        let spec = Spec::new(Kind::Forge, "1.20.1", "47.4.29").unwrap();
        let mut p = p();
        p["id"] = json!(spec.id());
        p["inheritsFrom"] = json!("1.20.1");
        p["mainClass"] = json!("com.evil.Payload");
        assert!(validate_profile(&p, &spec).is_err());
    }
    #[test]
    fn sidecar_parsing() {
        assert_eq!(
            parse_sha256_sidecar(&format!("{}  forge-installer.jar", "a".repeat(64))).unwrap(),
            "a".repeat(64)
        );
        assert_eq!(
            parse_sha256_sidecar(&format!("{}\n", "B".repeat(64))).unwrap(),
            "b".repeat(64)
        );
        assert!(parse_sha256_sidecar("too-short").is_err());
        assert!(parse_sha256_sidecar("").is_err());
        assert!(parse_sha256_sidecar(&"z".repeat(64)).is_err());
    }
}
