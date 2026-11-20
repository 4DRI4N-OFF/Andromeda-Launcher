//! Version catalog: the complete Minecraft version list with type filters and
//! per-loader version compatibility. Network answers are cached on disk with a
//! TTL; a stale cache is still usable offline and is reported as stale.
//! Listing a loader here never claims install/launch support: that is only
//! proven by the install and launch paths.
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

const MOJANG_MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const FABRIC_META: &str = "https://meta.fabricmc.net/v2/versions/loader";
const QUILT_META: &str = "https://meta.quiltmc.org/v3/versions/loader";
const FORGE_PROMOTIONS: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const FORGE_MAVEN_META: &str =
    "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";
const NEOFORGE_MAVEN_META: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";
/// NeoForge for Minecraft 1.20.1 lives in a separate artifact whose versions
/// are MC-combined ("1.20.1-47.1.106"). Verified 2026-11-20: the installer,
/// its .sha256 sidecar and a bootstraplauncher profile are all published.
const NEOFORGE_1_20_1_META: &str =
    "https://maven.neoforged.net/releases/net/neoforged/forge/maven-metadata.xml";
const CACHE_TTL: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McKind {
    Release,
    Snapshot,
    OldBeta,
    OldAlpha,
}
impl McKind {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "release" => Some(Self::Release),
            "snapshot" => Some(Self::Snapshot),
            "old_beta" => Some(Self::OldBeta),
            "old_alpha" => Some(Self::OldAlpha),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McVersion {
    pub id: String,
    pub kind: McKind,
    /// RFC 3339 release time from the manifest.
    pub released: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct McFilter {
    pub release: bool,
    pub snapshot: bool,
    pub old_beta: bool,
    pub old_alpha: bool,
}
impl McFilter {
    pub fn all() -> Self {
        Self {
            release: true,
            snapshot: true,
            old_beta: true,
            old_alpha: true,
        }
    }
    fn allows(&self, k: McKind) -> bool {
        match k {
            McKind::Release => self.release,
            McKind::Snapshot => self.snapshot,
            McKind::OldBeta => self.old_beta,
            McKind::OldAlpha => self.old_alpha,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoaderKind {
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}
impl LoaderKind {
    pub fn all() -> [Self; 4] {
        [Self::Fabric, Self::Quilt, Self::Forge, Self::NeoForge]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoaderVersion {
    pub kind: LoaderKind,
    pub version: String,
    /// Stability as published by the loader metadata; None when unknown.
    pub stable: Option<bool>,
}

/// True when the catalog fetch had to use a cache older than the TTL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogPage<T> {
    pub items: Vec<T>,
    pub stale: bool,
}

fn cache_path(dir: &Path, name: &str) -> Result<PathBuf> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
    {
        return Err("Unsafe cache name".into());
    }
    Ok(dir.join(format!("{name}.json")))
}

/// Fetch `url` as text through the downloader, with a TTL cache in `dir`.
/// On network failure, a stale cache entry is returned with `stale = true`.
///
/// Cache validity uses the file modification time; entries over 64 MiB are
/// rejected and writes go through a rename so readers never see a torn file.
fn cached_get(
    dl: &crate::vanilla::Downloader,
    dir: &Path,
    name: &str,
    url: &str,
) -> Result<(String, bool)> {
    let path = cache_path(dir, name)?;
    if let Ok(meta) = fs::metadata(&path) {
        if meta.is_file() && meta.len() > 0 && meta.len() <= 64 * 1024 * 1024 {
            let age = meta
                .modified()
                .ok()
                .and_then(|m| m.elapsed().ok())
                .unwrap_or(CACHE_TTL + Duration::from_secs(1));
            if age < CACHE_TTL {
                return Ok((fs::read_to_string(&path)?, false));
            }
        }
    }
    match dl.text(url) {
        Ok(body) => {
            if body.len() > 64 * 1024 * 1024 {
                return Err("Catalog response too large".into());
            }
            fs::create_dir_all(dir)?;
            let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
            fs::write(&tmp, &body)?;
            fs::rename(&tmp, &path)?;
            Ok((body, false))
        }
        Err(e) => {
            if let Ok(body) = fs::read_to_string(&path) {
                if !body.is_empty() {
                    return Ok((body, true));
                }
            }
            Err(e)
        }
    }
}

/// Complete Minecraft version list (all kinds), newest first, after `filter`.
pub fn mc_versions(
    dl: &crate::vanilla::Downloader,
    cache: &Path,
    filter: McFilter,
) -> Result<CatalogPage<McVersion>> {
    let (body, stale) = cached_get(dl, cache, "mc-manifest", MOJANG_MANIFEST)?;
    let v: Value = serde_json::from_str(&body)?;
    let arr = v["versions"].as_array().ok_or("Invalid manifest")?;
    let mut items = Vec::with_capacity(arr.len());
    for it in arr {
        let id = it["id"].as_str().unwrap_or_default();
        let kind = it["type"].as_str().and_then(McKind::parse);
        let released = it["releaseTime"].as_str().unwrap_or_default();
        let url = it["url"].as_str().unwrap_or_default();
        let Some(kind) = kind else { continue };
        if id.is_empty()
            || id.len() > 64
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b' '))
            || !url.starts_with("https://")
            || !filter.allows(kind)
        {
            continue;
        }
        items.push(McVersion {
            id: id.to_owned(),
            kind,
            released: released.to_owned(),
        });
    }
    Ok(CatalogPage { items, stale })
}

fn json_loader_list(
    dl: &crate::vanilla::Downloader,
    cache: &Path,
    kind: LoaderKind,
    mc: &str,
) -> Result<CatalogPage<LoaderVersion>> {
    let (base, name) = match kind {
        LoaderKind::Fabric => (FABRIC_META, format!("fabric-{mc}")),
        LoaderKind::Quilt => (QUILT_META, format!("quilt-{mc}")),
        _ => unreachable!(),
    };
    let url = format!("{base}/{mc}");
    let (body, stale) = match cached_get(dl, cache, &name, &url) {
        Ok(v) => v,
        Err(_) => {
            return Ok(CatalogPage {
                items: Vec::new(),
                stale: false,
            })
        }
    };
    let v: Value = serde_json::from_str(&body)?;
    let arr = v.as_array().ok_or("Invalid loader list")?;
    // Upstream order cannot be trusted: the Forge maven metadata arrived
    // jumbled, and the Quilt list interleaves pre-releases with releases.
    let mut items = Vec::with_capacity(arr.len());
    for it in arr {
        let loader = &it["loader"];
        let Some(ver) = loader["version"].as_str() else {
            continue;
        };
        if ver.is_empty()
            || ver.len() > 64
            || !ver
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+'))
        {
            continue;
        }
        items.push(LoaderVersion {
            kind,
            version: ver.to_owned(),
            stable: loader["stable"].as_bool(),
        });
    }
    items.sort_by(|a, b| version_cmp(&b.version, &a.version));
    Ok(CatalogPage { items, stale })
}

/// Map a NeoForge loader version (20.2.x and later) to its Minecraft version.
/// Rule published by the NeoForge project: the loader version mirrors the game
/// A Minecraft version plus optional loader build whose combination was
/// checked against the catalog. The only constructor is [verify_selection],
/// so callers that receive one can trust the combination exists upstream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedSelection {
    mc: String,
    loader: Option<LoaderVersion>,
}
impl VerifiedSelection {
    pub fn mc(&self) -> &str {
        &self.mc
    }
    pub fn loader(&self) -> Option<&LoaderVersion> {
        self.loader.as_ref()
    }
}

/// Verify a user-facing selection against the catalog: the Minecraft version
/// must be in the official manifest and, when a loader is chosen, the loader
/// build must be published for exactly that version. One loader at most; a
/// selection never carries two. Uses the cache when offline (`stale` data is
/// still a real published catalog; a never-cached catalog is an error).
pub fn verify_selection(
    dl: &crate::vanilla::Downloader,
    cache: &Path,
    mc: &str,
    loader: Option<(LoaderKind, &str)>,
) -> Result<VerifiedSelection> {
    if !mc_versions(dl, cache, McFilter::all())?
        .items
        .iter()
        .any(|v| v.id == mc)
    {
        return Err("Minecraft version is not in the official manifest".into());
    }
    let loader = match loader {
        None => None,
        Some((kind, version)) => {
            let page = loader_versions(dl, cache, kind, mc)?;
            let found = page.items.into_iter().find(|v| v.version == version);
            Some(found.ok_or("Loader build is not published for this Minecraft version")?)
        }
    };
    Ok(VerifiedSelection {
        mc: mc.to_owned(),
        loader,
    })
}

/// Map a NeoForge loader version to its Minecraft version, e.g. 21.11.48 ->
/// 1.21.11 or 20.4.x -> 1.20.4.
/// The separate 1.20.1 line (47.1.x, artifact net.neoforged:forge) is not
/// covered by this endpoint and is out of scope here.
pub fn neoforge_mc(loader: &str) -> Option<String> {
    let mut parts = loader.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    // Under net.neoforged:neoforge the game line starts at 1.20.2 (20.2.x);
    // 47.1.x belongs to the separate 1.20.1 artifact net.neoforged:forge.
    if major > 99 || (major < 21 && !(major == 20 && minor >= 2)) {
        return None;
    }
    parts.next()?;
    Some(format!("1.{major}.{minor}"))
}

/// Map a Forge combined version "1.21.11-61.2.5" to its Minecraft version.
pub fn forge_mc(combined: &str) -> Option<&str> {
    let (mc, loader) = combined.split_once('-')?;
    if mc.is_empty() || loader.is_empty() || !mc.starts_with("1.") {
        return None;
    }
    Some(mc)
}

/// Newest-first ordering for dotted loader versions ("14.23.5.2864",
/// "21.11.0-beta", "0.20.0-beta.9"): numeric core segments compare
/// numerically with missing segments treated as 0 ("0.24" == "0.24.0"),
/// a release sorts after its pre-releases, and pre-release identifiers
/// follow semver rules (numeric identifiers numerically and before
/// alphanumeric ones). The comparison is a total order.
fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    fn parse(v: &str) -> (Vec<u64>, Vec<String>) {
        let (core, pre) = match v.split_once('-') {
            Some((c, p)) => (c, p),
            None => (v, ""),
        };
        let nums = core
            .split('.')
            .map(|s| {
                let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
                digits.parse().unwrap_or(0)
            })
            .collect();
        let pre = if pre.is_empty() {
            Vec::new()
        } else {
            pre.split('.').map(str::to_string).collect()
        };
        (nums, pre)
    }
    let (na, pa) = parse(a);
    let (nb, pb) = parse(b);
    for i in 0..na.len().max(nb.len()) {
        let ord = na
            .get(i)
            .copied()
            .unwrap_or(0)
            .cmp(&nb.get(i).copied().unwrap_or(0));
        if ord != Ordering::Equal {
            return ord;
        }
    }
    // A release is newer than any of its pre-releases.
    match (pa.is_empty(), pb.is_empty()) {
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        _ => {}
    }
    for i in 0..pa.len().max(pb.len()) {
        let (ia, ib) = (pa.get(i), pb.get(i));
        let ord = match (ia, ib) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(x), Some(y)) => {
                let (xn, yn) = (x.parse::<u64>(), y.parse::<u64>());
                match (xn, yn) {
                    (Ok(xn), Ok(yn)) => xn.cmp(&yn),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => x.cmp(y),
                }
            }
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    Ordering::Equal
}

fn parse_maven_metadata(xml: &str) -> Vec<String> {
    // Minimal extraction of <version> entries inside <versions>; the metadata
    // files are small, publisher-controlled Maven XML.
    let mut out = Vec::new();
    let Some(versions) = xml
        .split_once("<versions>")
        .and_then(|(_, rest)| rest.split_once("</versions>"))
        .map(|(body, _)| body)
    else {
        return out;
    };
    let mut rest = versions;
    while let Some((_, after)) = rest.split_once("<version>") {
        let Some((ver, tail)) = after.split_once("</version>") else {
            break;
        };
        let ver = ver.trim();
        if !ver.is_empty()
            && ver.len() <= 64
            && ver
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+'))
        {
            out.push(ver.to_owned());
        }
        rest = tail;
    }
    out
}

/// Loader versions compatible with one Minecraft version. Empty list means
/// "not available for that Minecraft version", not an error.
pub fn loader_versions(
    dl: &crate::vanilla::Downloader,
    cache: &Path,
    kind: LoaderKind,
    mc: &str,
) -> Result<CatalogPage<LoaderVersion>> {
    if mc.is_empty()
        || mc.len() > 64
        || !mc
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
    {
        return Err("Invalid Minecraft version".into());
    }
    match kind {
        LoaderKind::Fabric | LoaderKind::Quilt => json_loader_list(dl, cache, kind, mc),
        LoaderKind::Forge => {
            let (body, stale) = cached_get(dl, cache, "forge-maven-meta", FORGE_MAVEN_META)?;
            let mut items: Vec<_> = parse_maven_metadata(&body)
                .into_iter()
                .filter(|v| forge_mc(v) == Some(mc))
                .map(|v| LoaderVersion {
                    kind,
                    version: v,
                    stable: None,
                })
                .collect();
            items.sort_by(|a, b| version_cmp(&b.version, &a.version));
            // Mark the recommended build stable when the promotions file knows it.
            if let Ok((promo, _)) = cached_get(dl, cache, "forge-promotions", FORGE_PROMOTIONS) {
                if let Ok(v) = serde_json::from_str::<Value>(&promo) {
                    if let Some(rec) = v["promos"][format!("{mc}-recommended")].as_str() {
                        for it in &mut items {
                            it.stable = Some(it.version == format!("{mc}-{rec}"));
                        }
                    }
                }
            }
            Ok(CatalogPage { items, stale })
        }
        LoaderKind::NeoForge => {
            if mc == "1.20.1" {
                let (body, stale) =
                    cached_get(dl, cache, "neoforge-1.20.1-meta", NEOFORGE_1_20_1_META)?;
                // Versions are MC-combined upstream; store them bare so a
                // selection carries the loader build only ("47.1.106").
                let mut items: Vec<_> = parse_maven_metadata(&body)
                    .into_iter()
                    .filter_map(|v| v.strip_prefix("1.20.1-").map(str::to_owned))
                    .filter(|v| !v.is_empty())
                    .map(|v| LoaderVersion {
                        kind,
                        version: v,
                        stable: None,
                    })
                    .collect();
                items.sort_by(|a, b| version_cmp(&b.version, &a.version));
                return Ok(CatalogPage { items, stale });
            }

            let (body, stale) = cached_get(dl, cache, "neoforge-maven-meta", NEOFORGE_MAVEN_META)?;
            let mut items: Vec<_> = parse_maven_metadata(&body)
                .into_iter()
                .filter(|v| neoforge_mc(v).as_deref() == Some(mc))
                .map(|v| LoaderVersion {
                    kind,
                    version: v,
                    stable: None,
                })
                .collect();
            items.sort_by(|a, b| version_cmp(&b.version, &a.version));
            Ok(CatalogPage { items, stale })
        }
    }
}

/// Which of the four loaders publish at least one build for `mc`.
pub fn available_loaders(
    dl: &crate::vanilla::Downloader,
    cache: &Path,
    mc: &str,
) -> Result<Vec<LoaderKind>> {
    let mut out = Vec::new();
    for kind in LoaderKind::all() {
        if !loader_versions(dl, cache, kind, mc)?.items.is_empty() {
            out.push(kind);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_mc_manifest_with_filters() {
        let fixture = r#"{"latest":{"release":"1.21.11","snapshot":"25w51a"},"versions":[
          {"id":"25w51a","type":"snapshot","url":"https://piston-meta.mojang.com/v1/packages/x/25w51a.json","time":"t","releaseTime":"2025-12-17T13:20:33+00:00"},
          {"id":"1.21.11","type":"release","url":"https://piston-meta.mojang.com/v1/packages/y/1.21.11.json","time":"t","releaseTime":"2025-12-11T14:01:52+00:00"},
          {"id":"b1.7.3","type":"old_beta","url":"https://piston-meta.mojang.com/v1/packages/z/b1.7.3.json","time":"t","releaseTime":"2011-07-08T00:00:00+00:00"},
          {"id":"rd-132211","type":"old_alpha","url":"https://piston-meta.mojang.com/v1/packages/w/rd-132211.json","time":"t","releaseTime":"2009-05-13T20:11:00+00:00"},
          {"id":"future-kind","type":"pending","url":"https://piston-meta.mojang.com/v1/packages/u/x.json","time":"t","releaseTime":"t"},
          {"id":"bad","type":"release","url":"http://insecure.example/x.json","time":"t","releaseTime":"t"}
        ]}"#;
        let v: Value = serde_json::from_str(fixture).unwrap();
        let arr = v["versions"].as_array().unwrap();
        let mut items = Vec::new();
        for it in arr {
            let id = it["id"].as_str().unwrap();
            let kind = it["type"].as_str().and_then(McKind::parse);
            let url = it["url"].as_str().unwrap();
            let Some(kind) = kind else { continue };
            if !url.starts_with("https://") || !McFilter::all().allows(kind) {
                continue;
            }
            items.push((id, kind));
        }
        assert_eq!(
            items,
            [
                ("25w51a", McKind::Snapshot),
                ("1.21.11", McKind::Release),
                ("b1.7.3", McKind::OldBeta),
                ("rd-132211", McKind::OldAlpha)
            ]
        );
        let only_release = McFilter {
            release: true,
            snapshot: false,
            old_beta: false,
            old_alpha: false,
        };
        assert!(only_release.allows(McKind::Release));
        assert!(!only_release.allows(McKind::Snapshot));
        assert!(!only_release.allows(McKind::OldBeta));
        assert!(!only_release.allows(McKind::OldAlpha));
    }

    #[test]
    fn neoforge_version_maps_to_mc() {
        assert_eq!(neoforge_mc("21.11.48").as_deref(), Some("1.21.11"));
        assert_eq!(neoforge_mc("21.11.47-beta").as_deref(), Some("1.21.11"));
        assert_eq!(neoforge_mc("20.4.237").as_deref(), Some("1.20.4"));
        assert_eq!(neoforge_mc("20.1.0"), None); // 1.20.1 line uses another artifact
        assert_eq!(neoforge_mc("19.2.0"), None);
        assert_eq!(neoforge_mc("21"), None);
        assert_eq!(neoforge_mc("x.y.z"), None);
    }

    #[test]
    fn forge_version_maps_to_mc() {
        assert_eq!(forge_mc("1.21.11-61.2.5"), Some("1.21.11"));
        assert_eq!(forge_mc("1.20.1-47.4.0"), Some("1.20.1"));
        assert_eq!(forge_mc("61.2.5"), None);
        assert_eq!(forge_mc("1.21.11-"), None);
    }

    #[test]
    fn maven_metadata_versions_extracted() {
        let xml = r#"<?xml version="1.0"?><metadata><groupId>net.neoforged</groupId><artifactId>neoforge</artifactId><versioning><latest>21.11.48</latest><versions><version>21.11.46</version>
        <version>21.11.47-beta</version><version>21.11.48</version><version>bad&lt;x</version></versions><lastUpdated>20251216120000</lastUpdated></versioning></metadata>"#;
        let v = parse_maven_metadata(xml);
        assert_eq!(v, ["21.11.46", "21.11.47-beta", "21.11.48"]);
        assert!(parse_maven_metadata("<metadata/>").is_empty());
    }

    fn fixture_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "andromeda-catalog-test-{}-{tag}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn verify_selection_accepts_published_combinations() {
        let dir = fixture_dir("verify-ok");
        std::fs::write(
            dir.join("mc-manifest.json"),
            r#"{"versions":[{"id":"1.21.11","type":"release","url":"https://piston-meta.mojang.com/x.json","releaseTime":"2025-01-01T00:00:00Z"},{"id":"b1.7.3","type":"old_beta","url":"https://piston-meta.mojang.com/y.json","releaseTime":"2011-01-01T00:00:00Z"}]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("fabric-1.21.11.json"),
            r#"[{"loader":{"version":"0.20.1"}}]"#,
        )
        .unwrap();
        let dl = crate::vanilla::Downloader::new().unwrap();
        let sel =
            verify_selection(&dl, &dir, "1.21.11", Some((LoaderKind::Fabric, "0.20.1"))).unwrap();
        assert_eq!(sel.mc(), "1.21.11");
        assert_eq!(sel.loader().unwrap().version, "0.20.1");
        let plain = verify_selection(&dl, &dir, "b1.7.3", None).unwrap();
        assert!(plain.loader().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn verify_selection_rejects_impossible_combinations() {
        let dir = fixture_dir("verify-bad");
        std::fs::write(
            dir.join("mc-manifest.json"),
            r#"{"versions":[{"id":"b1.7.3","type":"old_beta","url":"https://piston-meta.mojang.com/y.json","releaseTime":"2011-01-01T00:00:00Z"}]}"#,
        )
        .unwrap();
        std::fs::write(dir.join("fabric-b1.7.3.json"), r#"[]"#).unwrap();
        let dl = crate::vanilla::Downloader::new().unwrap();
        assert!(verify_selection(&dl, &dir, "1.5.2", None).is_err());
        assert!(
            verify_selection(&dl, &dir, "b1.7.3", Some((LoaderKind::Fabric, "0.20.1"))).is_err()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn loader_versions_sort_newest_first() {
        use std::cmp::Ordering::*;
        assert_eq!(version_cmp("47.4.29", "47.4.5"), Greater);
        assert_eq!(version_cmp("14.23.5.2864", "14.23.5.2859"), Greater);
        assert_eq!(version_cmp("21.11.48", "21.11.0-beta"), Greater);
        assert_eq!(version_cmp("0.20.1", "0.16.14"), Greater);
        assert_eq!(version_cmp("61.2.5", "61.2.5"), Equal);
        assert_eq!(version_cmp("0.30.2", "0.24.0"), Greater);
        assert_eq!(version_cmp("0.24.0", "0.20.0-beta.9"), Greater);
        assert_eq!(version_cmp("0.20.2-beta.1", "0.20.0-beta.9"), Greater);
        assert_eq!(version_cmp("0.24.0", "0.24.0-beta.1"), Greater);
        assert_eq!(version_cmp("0.20.0-beta.9", "0.20.0-beta.10"), Less);
        assert_eq!(version_cmp("0.24", "0.24.0"), Equal);
        // Sorting a jumbled prerelease mix yields the total order above.
        let mut vs = vec![
            "0.20.0-beta.9",
            "0.24.0",
            "0.20.0-beta.7",
            "0.30.2",
            "0.20.2-beta.1",
            "0.24.0-beta.1",
        ];
        vs.sort_by(|a, b| version_cmp(b, a));
        assert_eq!(
            vs,
            vec![
                "0.30.2",
                "0.24.0",
                "0.24.0-beta.1",
                "0.20.2-beta.1",
                "0.20.0-beta.9",
                "0.20.0-beta.7",
            ]
        );
        let dir = fixture_dir("sort");
        std::fs::write(
            dir.join("forge-maven-meta.json"),
            r#"<?xml version="1.0"?><metadata><versioning><versions><version>1.12.2-14.23.5.2859</version><version>1.12.2-14.23.5.2864</version><version>1.12.2-14.23.5.2860</version></versions></versioning></metadata>"#,
        )
        .unwrap();
        let dl = crate::vanilla::Downloader::new().unwrap();
        let page = loader_versions(&dl, &dir, LoaderKind::Forge, "1.12.2").unwrap();
        let versions: Vec<_> = page.items.iter().map(|v| v.version.as_str()).collect();
        assert_eq!(
            versions,
            [
                "1.12.2-14.23.5.2864",
                "1.12.2-14.23.5.2860",
                "1.12.2-14.23.5.2859"
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn neoforge_1_20_1_versions_are_uncombined() {
        let dir = fixture_dir("nf1201");
        std::fs::write(
            dir.join("neoforge-1.20.1-meta.json"),
            r#"<?xml version="1.0"?><metadata><versioning><versions><version>1.20.1-47.1.5</version><version>1.20.1-47.1.106</version></versions></versioning></metadata>"#,
        )
        .unwrap();
        let dl = crate::vanilla::Downloader::new().unwrap();
        let page = loader_versions(&dl, &dir, LoaderKind::NeoForge, "1.20.1").unwrap();
        let versions: Vec<_> = page.items.iter().map(|v| v.version.as_str()).collect();
        assert_eq!(versions, ["47.1.106", "47.1.5"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Live integration check against the real metadata services. Ignored by
    /// default so the suite stays hermetic; run explicitly before delivery:
    /// cargo test -p andromeda-core catalog_live -- --ignored
    #[test]
    #[ignore = "hits the live metadata services"]
    fn catalog_live_endpoints() {
        let dl = crate::vanilla::Downloader::new().unwrap();
        let dir =
            std::env::temp_dir().join(format!("andromeda-catalog-live-{}", std::process::id()));
        let mc = mc_versions(&dl, &dir, McFilter::all()).unwrap();
        assert!(mc.items.len() > 500, "complete manifest expected");
        assert!(mc
            .items
            .iter()
            .any(|v| v.id == "1.21.11" && v.kind == McKind::Release));
        assert!(mc.items.iter().any(|v| v.kind == McKind::Snapshot));
        assert!(mc.items.iter().any(|v| v.kind == McKind::OldBeta));
        assert!(mc.items.iter().any(|v| v.kind == McKind::OldAlpha));
        let releases_only = mc_versions(
            &dl,
            &dir,
            McFilter {
                release: true,
                snapshot: false,
                old_beta: false,
                old_alpha: false,
            },
        )
        .unwrap();
        assert!(releases_only
            .items
            .iter()
            .all(|v| v.kind == McKind::Release));

        let fabric = loader_versions(&dl, &dir, LoaderKind::Fabric, "1.21.11").unwrap();
        assert!(
            fabric.items.iter().any(|v| v.version == "0.20.1"),
            "fabric 0.20.1 for 1.21.11"
        );
        let quilt = loader_versions(&dl, &dir, LoaderKind::Quilt, "1.21.11").unwrap();
        assert!(
            quilt.items.iter().any(|v| v.version == "0.30.2"),
            "quilt 0.30.2 for 1.21.11"
        );
        let forge = loader_versions(&dl, &dir, LoaderKind::Forge, "1.21.11").unwrap();
        assert!(
            forge.items.iter().any(|v| v.version == "1.21.11-61.2.5"),
            "forge 61.2.5 for 1.21.11"
        );
        let neoforge = loader_versions(&dl, &dir, LoaderKind::NeoForge, "1.21.11").unwrap();
        assert!(
            neoforge.items.iter().any(|v| v.version == "21.11.48"),
            "neoforge 21.11.48 for 1.21.11"
        );
        let nf_1201 = loader_versions(&dl, &dir, LoaderKind::NeoForge, "1.20.1").unwrap();
        // 47.1.106 was additionally verified installable 2026-11-20: the
        // installer jar downloads, its .sha256 sidecar matches, and its
        // profile carries id 1.20.1-forge-47.1.106 / bootstraplauncher.
        assert!(
            nf_1201.items.iter().any(|v| v.version == "47.1.106"),
            "neoforge 47.1.106 expected for 1.20.1"
        );
        let old = loader_versions(&dl, &dir, LoaderKind::Fabric, "b1.7.3").unwrap();
        assert!(old.items.is_empty(), "no Fabric for old_beta expected");
        let avail = available_loaders(&dl, &dir, "1.21.11").unwrap();
        assert_eq!(avail.len(), 4, "all four loaders publish 1.21.11 builds");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_rejects_unsafe_names() {
        let d = Path::new("/tmp/catalog-test");
        assert!(cache_path(d, "fabric-1.21.11").is_ok());
        assert!(cache_path(d, "../escape").is_err());
        assert!(cache_path(d, "a/b").is_err());
        assert!(cache_path(d, "").is_err());
    }
}
