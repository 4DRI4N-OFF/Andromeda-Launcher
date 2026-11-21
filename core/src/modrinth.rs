//! Modrinth metadata: search, per-project compatible versions, and safe
//! dependency resolution for mod installs.
//!
//! Source of truth: the Modrinth v2 API at https://api.modrinth.com/v2
//! (anonymous, no token). Search facets use `categories:<loader facet>`
//! (loaders are categories in search) plus `project_type:mod`; the exact
//! Minecraft-version filter is applied when listing a project's versions
//! with `loaders=[...]&game_versions=[...]`.
//!
//! Verified live 2026-11-20: loader facets exist for every Andromeda
//! loader (fabric, quilt, forge, neoforge, legacy-fabric, babric, ornithe,
//! liteloader). Version files carry sha1+sha512 digests, sizes, and
//! cdn.modrinth.com URLs; the sha512 from the API authenticates downloads
//! (TLS, same-origin metadata trust model as the loader installers).
//!
//! Only the parsing/validation/selection/planning lives here and is
//! hermetically tested; the thin network wrappers reuse the catalog's
//! TTL cache and are exercised by live tests marked `#[ignore]`.

use serde_json::Value;
use std::path::Path;

use crate::catalog::LoaderKind;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub const API: &str = "https://api.modrinth.com/v2";
/// Digests come from the API, files from this host only.
const CDN_PREFIX: &str = "https://cdn.modrinth.com/data/";
/// Resolution depth and breadth caps keep hostile or broken dependency
/// graphs from exploding an install.
const MAX_DEPTH: usize = 8;
const MAX_PLANNED: usize = 32;
const MAX_VERSIONS: usize = 512;
const MAX_FILES: usize = 32;
const MAX_DEPS: usize = 64;
const MAX_HITS: usize = 64;

/// The Modrinth loader facet for an Andromeda loader.
pub fn facet(kind: LoaderKind) -> &'static str {
    match kind {
        LoaderKind::Fabric => "fabric",
        LoaderKind::Quilt => "quilt",
        LoaderKind::Forge => "forge",
        LoaderKind::NeoForge => "neoforge",
        LoaderKind::LegacyFabric => "legacy-fabric",
        LoaderKind::Babric => "babric",
        LoaderKind::Ornithe => "ornithe",
        LoaderKind::LiteLoader => "liteloader",
    }
}

/// Project or version id: Modrinth base62 ids, observed length 8.
fn id_valid(s: &str) -> bool {
    (6..=16).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn name_valid(s: &str) -> bool {
    // Display strings (titles, slugs, version names): they never reach the
    // filesystem (disk names come from filename_valid), so slashes are fine;
    // reject only control characters and overlong values.
    !s.is_empty() && s.len() <= 120 && s.bytes().all(|b| !matches!(b, 0..=0x1f))
}

/// Mod file names must be plain archives with no path components.
fn filename_valid(s: &str) -> bool {
    (s.ends_with(".jar") || s.ends_with(".litemod"))
        && s.len() <= 140
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+'))
}

/// Public filename check for cross-module destination validation.
pub fn file_name_ok(s: &str) -> bool {
    filename_valid(s)
}

fn hex_valid(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn url_valid(s: &str) -> bool {
    s.starts_with(CDN_PREFIX) && s.len() <= 512 && !s.bytes().any(|b| b == b' ' || b < 0x21)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    Release,
    Beta,
    Alpha,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepKind {
    Required,
    Optional,
    Incompatible,
    Embedded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dependency {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub kind: DepKind,
}

/// One downloadable file of a version, digests as published by the API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModFile {
    pub url: String,
    pub filename: String,
    pub size: u64,
    pub sha1: String,
    pub sha512: String,
    pub primary: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub number: String,
    pub release: Release,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    /// RFC3339 publish date, used only for ordering.
    pub date: String,
    pub files: Vec<ModFile>,
    pub deps: Vec<Dependency>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub downloads: u64,
}

fn parse_file(v: &Value) -> Result<ModFile> {
    let url = v["url"].as_str().ok_or("Mod file missing url")?;
    let filename = v["filename"].as_str().ok_or("Mod file missing filename")?;
    let hashes = &v["hashes"];
    let sha1 = hashes["sha1"].as_str().ok_or("Mod file missing sha1")?;
    let sha512 = hashes["sha512"].as_str().ok_or("Mod file missing sha512")?;
    let size = v["size"].as_u64().ok_or("Mod file missing size")?;
    if !url_valid(url) {
        return Err("Mod file url is not on the pinned CDN".into());
    }
    if !filename_valid(filename) {
        return Err("Unsafe Modrinth file name".into());
    }
    if !hex_valid(sha1, 40) || !hex_valid(sha512, 128) {
        return Err("Invalid Modrinth file digest".into());
    }
    if size == 0 || size > 512 * 1024 * 1024 {
        return Err("Invalid Modrinth file size".into());
    }
    Ok(ModFile {
        url: url.into(),
        filename: filename.into(),
        size,
        sha1: sha1.into(),
        sha512: sha512.into(),
        primary: v["primary"].as_bool().unwrap_or(false),
    })
}

fn parse_dependency(v: &Value) -> Result<Dependency> {
    let kind = match v["dependency_type"].as_str() {
        Some("required") => DepKind::Required,
        Some("optional") => DepKind::Optional,
        Some("incompatible") => DepKind::Incompatible,
        Some("embedded") => DepKind::Embedded,
        _ => return Err("Unknown Modrinth dependency type".into()),
    };
    let project_id = v["project_id"].as_str().map(str::to_owned);
    let version_id = v["version_id"].as_str().map(str::to_owned);
    if let Some(p) = &project_id {
        if !id_valid(p) {
            return Err("Invalid Modrinth dependency project id".into());
        }
    }
    if let Some(vid) = &version_id {
        if !id_valid(vid) {
            return Err("Invalid Modrinth dependency version id".into());
        }
    }
    if project_id.is_none() && version_id.is_none() {
        return Err("Modrinth dependency without target".into());
    }
    Ok(Dependency {
        project_id,
        version_id,
        kind,
    })
}

/// Parse and validate the version list returned for one project. The list
/// must already be loader/MC-filtered by the request; each entry is
/// re-checked against `facet` and `mc` so a stale or hostile cache entry
/// can never install for the wrong game.
pub fn parse_versions(
    body: &Value,
    project_id: &str,
    facet: &str,
    mc: &str,
) -> Result<Vec<ModVersion>> {
    if !id_valid(project_id) {
        return Err("Invalid Modrinth project id".into());
    }
    crate::instances::validate_mc_id(mc)?;
    let list = body.as_array().ok_or("Modrinth versions must be a list")?;
    if list.len() > MAX_VERSIONS {
        return Err("Too many Modrinth versions".into());
    }
    let mut out = Vec::new();
    for v in list {
        let id = v["id"].as_str().ok_or("Mod version missing id")?;
        let pid = v["project_id"]
            .as_str()
            .ok_or("Mod version missing project")?;
        if !id_valid(id) || pid != project_id {
            return Err("Mod version id mismatch".into());
        }
        let name = v["name"].as_str().unwrap_or("").to_owned();
        let number = v["version_number"]
            .as_str()
            .ok_or("Mod version missing number")?;
        if !name_valid(&name) || !name_valid(number) {
            return Err("Unsafe Modrinth version label".into());
        }
        let release = match v["version_type"].as_str() {
            Some("release") => Release::Release,
            Some("beta") => Release::Beta,
            Some("alpha") => Release::Alpha,
            _ => return Err("Unknown Modrinth release channel".into()),
        };
        let game_versions: Vec<String> = v["game_versions"]
            .as_array()
            .ok_or("Mod version missing game versions")?
            .iter()
            .filter_map(|g| g.as_str().map(str::to_owned))
            .collect();
        let loaders: Vec<String> = v["loaders"]
            .as_array()
            .ok_or("Mod version missing loaders")?
            .iter()
            .filter_map(|g| g.as_str().map(str::to_owned))
            .collect();
        if !game_versions.iter().any(|g| g == mc) || !loaders.iter().any(|l| l == facet) {
            continue; // not applicable to this instance; skip, don't fail the list
        }
        let files_v = v["files"].as_array().ok_or("Mod version missing files")?;
        if files_v.is_empty() || files_v.len() > MAX_FILES {
            return Err("Invalid Modrinth file count".into());
        }
        let mut files = Vec::new();
        for f in files_v {
            files.push(parse_file(f)?);
        }
        let deps_v = v["dependencies"]
            .as_array()
            .ok_or("Mod version missing dependencies")?;
        if deps_v.len() > MAX_DEPS {
            return Err("Too many Modrinth dependencies".into());
        }
        let mut deps = Vec::new();
        for d in deps_v {
            deps.push(parse_dependency(d)?);
        }
        out.push(ModVersion {
            id: id.into(),
            project_id: pid.into(),
            name,
            number: number.into(),
            release,
            game_versions,
            loaders,
            date: v["date_published"].as_str().unwrap_or("").into(),
            files,
            deps,
        });
    }
    Ok(out)
}

/// Parse the search response into validated project summaries.
pub fn parse_search(body: &Value) -> Result<Vec<Project>> {
    let hits = body["hits"]
        .as_array()
        .ok_or("Modrinth search missing hits")?;
    if hits.len() > MAX_HITS {
        return Err("Too many Modrinth search hits".into());
    }
    let mut out = Vec::new();
    for h in hits {
        let id = h["project_id"].as_str().ok_or("Search hit missing id")?;
        let slug = h["slug"].as_str().unwrap_or("");
        let title = h["title"].as_str().ok_or("Search hit missing title")?;
        if !id_valid(id) || !name_valid(title) || !name_valid(slug) {
            return Err("Unsafe Modrinth search hit".into());
        }
        if h["project_type"].as_str() != Some("mod") {
            continue;
        }
        out.push(Project {
            id: id.into(),
            slug: slug.into(),
            title: title.into(),
            description: h["description"].as_str().unwrap_or("").into(),
            downloads: h["downloads"].as_u64().unwrap_or(0),
        });
    }
    Ok(out)
}

/// The file to install for a version: the primary file, or the only file.
pub fn primary_file(v: &ModVersion) -> Result<&ModFile> {
    let primaries: Vec<&ModFile> = v.files.iter().filter(|f| f.primary).collect();
    match (primaries.len(), v.files.len()) {
        (1, _) => Ok(primaries[0]),
        (0, 1) => Ok(&v.files[0]),
        _ => Err("Modrinth version has no clear primary file".into()),
    }
}

/// Preferred version for install: newest release, else newest beta, else
/// newest alpha. The API returns newest first; `unstable` flags the
/// fallback so callers can label the choice.
pub fn pick(versions: &[ModVersion]) -> Option<(&ModVersion, bool)> {
    for want in [Release::Release, Release::Beta, Release::Alpha] {
        if let Some(v) = versions.iter().find(|v| v.release == want) {
            return Some((v, want != Release::Release));
        }
    }
    None
}

/// What the registry knows about an already-installed project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    pub version_id: String,
    pub mc: String,
    pub loader: String,
    pub enabled: bool,
}

/// One resolved install step, dependencies before dependents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Planned {
    pub project_id: String,
    pub version_id: String,
    pub version_number: String,
    pub title: String,
    pub file: ModFile,
    /// Project that pulled this in as a required dependency, if any.
    pub required_by: Option<String>,
    pub unstable: bool,
    /// True when this step replaces a different installed build.
    pub replaces: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub installs: Vec<Planned>,
    /// Installed-but-disabled projects the plan needs switched back on.
    pub enable: Vec<String>,
    /// Optional dependencies surfaced as suggestions (project id + label).
    pub optional: Vec<(String, String)>,
}

/// Resolve an install plan for `target` (the chosen version of the
/// requested project) on instance `mc`+`facet`.
///
/// - required dependencies are resolved recursively, pinned to the declared
///   version when given, else the preferred compatible build;
/// - a required dependency with no compatible build fails the whole plan —
///   a jar without its required mods is not a functional install;
/// - an `incompatible` entry that matches an installed project aborts the
///   plan with a conflict error instead of breaking the instance;
/// - optional dependencies are reported, never installed;
/// - embedded dependencies are bundled in the jar: nothing to do;
/// - `installed` answers the registry lookup by project id; `versions_of`
///   lists compatible versions for a project (caller filters loader+MC).
pub fn plan_install(
    target: &ModVersion,
    mc: &str,
    facet: &str,
    installed: &dyn Fn(&str) -> Option<Installed>,
    versions_of: &dyn Fn(&str) -> Result<Vec<ModVersion>>,
) -> Result<Plan> {
    let mut plan = Plan::default();
    // Project id -> build the plan (or a pinned requirement) commits to.
    // A second, different pinned build for the same project is a
    // dependency conflict, not a silent first-wins.
    let mut committed: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    // (version to process, required_by, depth)
    let mut queue: Vec<(ModVersion, Option<String>, usize)> = vec![(target.clone(), None, 0usize)];
    while let Some((version, required_by, depth)) = queue.pop() {
        if depth > MAX_DEPTH {
            return Err("Modrinth dependency graph too deep".into());
        }
        let pid = version.project_id.clone();
        if let Some(prev) = committed.get(&pid) {
            if prev != &version.id {
                return Err(format!(
                    "Dependency conflict: {pid} is required as two different builds"
                )
                .into());
            }
            continue; // already processed at this exact build
        }
        committed.insert(pid.clone(), version.id.clone());
        // Conflicts: any incompatible entry that names an installed project
        // aborts the install before anything is written.
        for d in &version.deps {
            if d.kind == DepKind::Incompatible {
                if let Some(p) = &d.project_id {
                    if installed(p).is_some() {
                        return Err(format!(
                            "Conflict: {pid} declares project {p} incompatible and it is installed"
                        )
                        .into());
                    }
                }
            }
        }
        let mut install_step = true;
        if let Some(inst) = installed(&pid) {
            if inst.mc == mc && inst.loader == facet && inst.version_id == version.id {
                if inst.enabled {
                    install_step = false; // exact build already active
                } else {
                    // Right build, switched off: re-enable instead of
                    // reinstalling; its dependencies are re-validated below.
                    if !plan.enable.contains(&pid) {
                        plan.enable.push(pid.clone());
                    }
                    install_step = false;
                }
            } else {
                // Different or inapplicable build: replace it.
                let replaces = Some(inst.version_id.clone());
                let file = primary_file(&version)?.clone();
                plan.installs.push(Planned {
                    project_id: pid.clone(),
                    version_id: version.id.clone(),
                    version_number: version.number.clone(),
                    title: version.name.clone(),
                    file,
                    required_by: required_by.clone(),
                    unstable: version.release != Release::Release,
                    replaces,
                });
                install_step = false;
            }
        }
        if install_step {
            plan.installs.push(Planned {
                project_id: pid.clone(),
                version_id: version.id.clone(),
                version_number: version.number.clone(),
                title: version.name.clone(),
                file: primary_file(&version)?.clone(),
                required_by: required_by.clone(),
                unstable: version.release != Release::Release,
                replaces: None,
            });
        }
        if plan.installs.len() > MAX_PLANNED {
            return Err("Modrinth dependency graph too large".into());
        }
        for d in &version.deps {
            match d.kind {
                DepKind::Required => {
                    let dep_pid = d
                        .project_id
                        .clone()
                        .ok_or("Required dependency names only a version")?;
                    // A pinned build that disagrees with what the plan (or
                    // the instance) already committed to is a conflict even
                    // before resolution.
                    if let Some(vid) = &d.version_id {
                        if let Some(prev) = committed.get(&dep_pid) {
                            if prev != vid {
                                return Err(format!(
                                    "Dependency conflict: {dep_pid} is required as two different builds"
                                )
                                .into());
                            }
                            continue;
                        }
                    } else if committed.contains_key(&dep_pid) {
                        continue;
                    }
                    let candidates = versions_of(&dep_pid)?;
                    let chosen = if let Some(vid) = &d.version_id {
                        candidates
                            .iter()
                            .find(|v| &v.id == vid)
                            .ok_or_else(|| {
                                format!("Required dependency {dep_pid} has no compatible build")
                            })?
                            .clone()
                    } else {
                        pick(&candidates)
                            .ok_or_else(|| {
                                format!("Required dependency {dep_pid} has no compatible build")
                            })?
                            .0
                            .clone()
                    };
                    queue.push((chosen, Some(pid.clone()), depth + 1));
                }
                DepKind::Optional => {
                    if let Some(p) = &d.project_id {
                        plan.optional.push((p.clone(), pid.clone()));
                    }
                }
                DepKind::Incompatible | DepKind::Embedded => {}
            }
        }
    }
    // Emit dependencies before the mods that need them.
    plan.installs.reverse();
    Ok(plan)
}

/// Reverse-direction conflict check: metadata of the builds already
/// installed can declare the NEW project incompatible. `installed_deps`
/// pairs each installed project id with the dependency list of its
/// installed build (fetched fresh by the caller); `incoming` are the
/// project ids about to be installed. Pure, so it is testable hermetically.
pub fn reverse_conflict(
    installed_deps: &[(String, Vec<Dependency>)],
    incoming: &[String],
) -> Result<()> {
    for (pid, deps) in installed_deps {
        for d in deps {
            if d.kind == DepKind::Incompatible {
                if let Some(p) = &d.project_id {
                    if incoming.iter().any(|n| n == p) {
                        return Err(format!(
                            "Conflict: installed project {pid} declares {p} incompatible"
                        )
                        .into());
                    }
                }
            }
        }
    }
    Ok(())
}

/// Search URL for mods matching `query` on a loader facet.
pub fn search_url(facet: &str, query: &str, limit: usize) -> String {
    let facets = format!("[[\"project_type:mod\"],[\"categories:{facet}\"]]");
    let q = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("query", query)
        .append_pair("limit", &limit.min(MAX_HITS).to_string())
        .append_pair("facets", &facets)
        .append_pair("index", "relevance")
        .finish();
    format!("{API}/search?{q}")
}

/// Compatible-versions URL for one project on one instance.
pub fn versions_url(project_id: &str, facet: &str, mc: &str) -> String {
    let q = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("loaders", &format!("[\"{facet}\"]"))
        .append_pair("game_versions", &format!("[\"{mc}\"]"))
        .finish();
    format!("{API}/project/{project_id}/version?{q}")
}

/// Live search through the shared TTL cache. Returns (projects, stale).
pub fn search(
    dl: &crate::vanilla::Downloader,
    cache: &Path,
    facet: &str,
    query: &str,
) -> Result<(Vec<Project>, bool)> {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(format!("search|{facet}|{query}").as_bytes());
    let hex: String = digest[..8].iter().map(|b| format!("{b:02x}")).collect();
    let name = format!("modrinth-{hex}");
    let (body, stale) =
        crate::catalog::cached_get(dl, cache, &name, &search_url(facet, query, 24))?;
    Ok((parse_search(&serde_json::from_str(&body)?)?, stale))
}

/// Live compatible-version listing through the shared TTL cache.
pub fn versions(
    dl: &crate::vanilla::Downloader,
    cache: &Path,
    project_id: &str,
    facet: &str,
    mc: &str,
) -> Result<(Vec<ModVersion>, bool)> {
    let name = format!("modrinth-v-{project_id}-{facet}-{mc}");
    let (body, stale) =
        crate::catalog::cached_get(dl, cache, &name, &versions_url(project_id, facet, mc))?;
    Ok((
        parse_versions(&serde_json::from_str(&body)?, project_id, facet, mc)?,
        stale,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn file(primary: bool) -> Value {
        json!({
            "url": "https://cdn.modrinth.com/data/AANobbMI/versions/x/mod-1.0.jar",
            "filename": "mod-1.0.jar",
            "size": 1024,
            "primary": primary,
            "hashes": {"sha1": "a".repeat(40), "sha512": "b".repeat(128)}
        })
    }

    fn version(id: &str, project: &str, release: &str, deps: Value) -> Value {
        json!({
            "id": id,
            "project_id": project,
            "name": format!("Mod {id}"),
            "version_number": "1.0",
            "version_type": release,
            "game_versions": ["1.12.2"],
            "loaders": ["forge"],
            "date_published": "2026-01-01T00:00:00Z",
            "files": [file(true)],
            "dependencies": deps
        })
    }

    #[test]
    fn parses_versions_and_skips_inapplicable() {
        let mut beta = version("BBBBbbbb", "PROJproj", "beta", json!([]));
        beta["game_versions"] = json!(["1.20.1"]);
        let body = json!([version("AAAAaaaa", "PROJproj", "release", json!([])), beta,]);
        let vs = parse_versions(&body, "PROJproj", "forge", "1.12.2").unwrap();
        assert_eq!(vs.len(), 1);
        assert_eq!(vs[0].id, "AAAAaaaa");
    }

    #[test]
    fn rejects_off_cdn_url_and_bad_names() {
        let mut v = version("AAAAaaaa", "PROJproj", "release", json!([]));
        v["files"][0]["url"] = json!("https://evil.example/x.jar");
        assert!(parse_versions(&json!([v]), "PROJproj", "forge", "1.12.2").is_err());
        let mut v = version("AAAAaaaa", "PROJproj", "release", json!([]));
        v["files"][0]["filename"] = json!("../escape.jar");
        assert!(parse_versions(&json!([v]), "PROJproj", "forge", "1.12.2").is_err());
        let mut v = version("AAAAaaaa", "PROJproj", "release", json!([]));
        v["files"][0]["hashes"]["sha512"] = json!("zz");
        assert!(parse_versions(&json!([v]), "PROJproj", "forge", "1.12.2").is_err());
    }

    #[test]
    fn pick_prefers_release_then_beta_then_alpha() {
        let body = json!([
            version("AAAAaaaa", "PROJproj", "alpha", json!([])),
            version("BBBBbbbb", "PROJproj", "beta", json!([])),
            version("CCCCcccc", "PROJproj", "release", json!([])),
        ]);
        let vs = parse_versions(&body, "PROJproj", "forge", "1.12.2").unwrap();
        assert_eq!(pick(&vs).unwrap().0.id, "CCCCcccc");
        let vs: Vec<_> = vs.into_iter().filter(|v| v.id != "CCCCcccc").collect();
        assert_eq!(pick(&vs).unwrap().0.id, "BBBBbbbb");
        assert!(
            pick(&vs).unwrap().1,
            "beta fallback must be flagged unstable"
        );
    }

    fn no_installed(_: &str) -> Option<Installed> {
        None
    }

    #[test]
    fn plan_resolves_required_chain_dependencies_first() {
        // target DEP0 requires LIB1 (any build); LIB1 requires LIB2 pinned.
        let dep = |p: &str, v: Option<&str>| json!({"project_id": p, "version_id": v, "dependency_type": "required"});
        let target = parse_versions(
            &json!([version(
                "TGT0tgt0",
                "ROOTroot",
                "release",
                json!([dep("LIB1lib1", None)])
            )]),
            "ROOTroot",
            "forge",
            "1.12.2",
        )
        .unwrap()
        .remove(0);
        let versions_of = |p: &str| -> Result<Vec<ModVersion>> {
            let body = match p {
                "LIB1lib1" => json!([version(
                    "L1v1l1v1",
                    "LIB1lib1",
                    "release",
                    json!([dep("LIB2lib2", Some("L2v2l2v2"))])
                )]),
                "LIB2lib2" => json!([version("L2v2l2v2", "LIB2lib2", "release", json!([]))]),
                _ => json!([]),
            };
            parse_versions(&body, p, "forge", "1.12.2")
        };
        let plan = plan_install(&target, "1.12.2", "forge", &no_installed, &versions_of).unwrap();
        let order: Vec<&str> = plan
            .installs
            .iter()
            .map(|p| p.project_id.as_str())
            .collect();
        assert_eq!(order, ["LIB2lib2", "LIB1lib1", "ROOTroot"]);
        assert_eq!(plan.installs[0].required_by.as_deref(), Some("LIB1lib1"));
        assert_eq!(plan.installs[2].required_by, None);
    }

    #[test]
    fn plan_fails_when_required_dep_has_no_compatible_build() {
        let dep =
            json!({"project_id": "LIB9lib9", "version_id": null, "dependency_type": "required"});
        let target = parse_versions(
            &json!([version("TGT0tgt0", "ROOTroot", "release", json!([dep]))]),
            "ROOTroot",
            "forge",
            "1.12.2",
        )
        .unwrap()
        .remove(0);
        let versions_of = |_: &str| -> Result<Vec<ModVersion>> { Ok(vec![]) };
        assert!(plan_install(&target, "1.12.2", "forge", &no_installed, &versions_of).is_err());
    }

    #[test]
    fn plan_aborts_on_installed_incompatible() {
        let dep = json!({"project_id": "BAD0bad0", "version_id": null, "dependency_type": "incompatible"});
        let target = parse_versions(
            &json!([version("TGT0tgt0", "ROOTroot", "release", json!([dep]))]),
            "ROOTroot",
            "forge",
            "1.12.2",
        )
        .unwrap()
        .remove(0);
        let installed = |p: &str| {
            (p == "BAD0bad0").then(|| Installed {
                version_id: "whatever".into(),
                mc: "1.12.2".into(),
                loader: "forge".into(),
                enabled: true,
            })
        };
        let versions_of = |_: &str| -> Result<Vec<ModVersion>> { Ok(vec![]) };
        let err = plan_install(&target, "1.12.2", "forge", &installed, &versions_of)
            .unwrap_err()
            .to_string();
        assert!(err.contains("incompatible"), "{err}");
    }

    #[test]
    fn plan_surfaces_optional_without_installing() {
        let deps = json!([
            {"project_id": "OPT1opt1", "version_id": null, "dependency_type": "optional"},
            {"project_id": "EMB1emb1", "version_id": null, "dependency_type": "embedded"}
        ]);
        let target = parse_versions(
            &json!([version("TGT0tgt0", "ROOTroot", "release", deps)]),
            "ROOTroot",
            "forge",
            "1.12.2",
        )
        .unwrap()
        .remove(0);
        let versions_of = |_: &str| -> Result<Vec<ModVersion>> { Ok(vec![]) };
        let plan = plan_install(&target, "1.12.2", "forge", &no_installed, &versions_of).unwrap();
        assert_eq!(plan.installs.len(), 1);
        assert_eq!(
            plan.optional,
            [("OPT1opt1".to_string(), "ROOTroot".to_string())]
        );
    }

    #[test]
    fn plan_skips_exact_installed_build_and_updates_stale_one() {
        let target = parse_versions(
            &json!([version("TGT0tgt0", "ROOTroot", "release", json!([]))]),
            "ROOTroot",
            "forge",
            "1.12.2",
        )
        .unwrap()
        .remove(0);
        let versions_of = |_: &str| -> Result<Vec<ModVersion>> { Ok(vec![]) };
        let same = |_: &str| {
            Some(Installed {
                version_id: "TGT0tgt0".into(),
                mc: "1.12.2".into(),
                loader: "forge".into(),
                enabled: true,
            })
        };
        let plan = plan_install(&target, "1.12.2", "forge", &same, &versions_of).unwrap();
        assert!(
            plan.installs.is_empty(),
            "exact installed build needs no work"
        );
        assert!(plan.enable.is_empty());
        let stale = |_: &str| {
            Some(Installed {
                version_id: "OLD0old0".into(),
                mc: "1.12.2".into(),
                loader: "forge".into(),
                enabled: true,
            })
        };
        let plan = plan_install(&target, "1.12.2", "forge", &stale, &versions_of).unwrap();
        assert_eq!(plan.installs[0].replaces.as_deref(), Some("OLD0old0"));
    }

    #[test]
    fn plan_breaks_dependency_cycles() {
        let dep =
            |p: &str| json!({"project_id": p, "version_id": null, "dependency_type": "required"});
        let target = parse_versions(
            &json!([version(
                "TGT0tgt0",
                "ROOTroot",
                "release",
                json!([dep("CYC1cyc1")])
            )]),
            "ROOTroot",
            "forge",
            "1.12.2",
        )
        .unwrap()
        .remove(0);
        let versions_of = |p: &str| -> Result<Vec<ModVersion>> {
            let body = match p {
                // cycle back to ROOTroot
                "CYC1cyc1" => json!([version(
                    "C1v1c1v1",
                    "CYC1cyc1",
                    "release",
                    json!([dep("ROOTroot")])
                )]),
                "ROOTroot" => json!([version("TGT0tgt0", "ROOTroot", "release", json!([]))]),
                _ => json!([]),
            };
            parse_versions(&body, p, "forge", "1.12.2")
        };
        let plan = plan_install(&target, "1.12.2", "forge", &no_installed, &versions_of).unwrap();
        assert_eq!(plan.installs.len(), 2, "cycle must terminate");
    }

    #[test]
    fn disabled_installed_build_is_enabled_not_reinstalled() {
        let target = parse_versions(
            &json!([version("TGT0tgt0", "ROOTroot", "release", json!([]))]),
            "ROOTroot",
            "forge",
            "1.12.2",
        )
        .unwrap()
        .remove(0);
        let versions_of = |_: &str| -> Result<Vec<ModVersion>> { Ok(vec![]) };
        let disabled = |_: &str| {
            Some(Installed {
                version_id: "TGT0tgt0".into(),
                mc: "1.12.2".into(),
                loader: "forge".into(),
                enabled: false,
            })
        };
        let plan = plan_install(&target, "1.12.2", "forge", &disabled, &versions_of).unwrap();
        assert!(plan.installs.is_empty(), "no download for the right build");
        assert_eq!(plan.enable, ["ROOTroot".to_string()]);
    }

    #[test]
    fn two_pinned_builds_of_one_dependency_conflict() {
        let dep = |p: &str, v: Option<&str>| json!({"project_id": p, "version_id": v, "dependency_type": "required"});
        // ROOT requires LIB1 pinned L1v1 and LIB2; LIB2 requires LIB1 pinned OTHER.
        let target = parse_versions(
            &json!([version(
                "TGT0tgt0",
                "ROOTroot",
                "release",
                json!([dep("LIB1lib1", Some("L1v1l1v1")), dep("LIB2lib2", None)])
            )]),
            "ROOTroot",
            "forge",
            "1.12.2",
        )
        .unwrap()
        .remove(0);
        let versions_of = |p: &str| -> Result<Vec<ModVersion>> {
            let body = match p {
                "LIB1lib1" => json!([
                    version("L1v1l1v1", "LIB1lib1", "release", json!([])),
                    version("OTHERoth", "LIB1lib1", "release", json!([])),
                ]),
                "LIB2lib2" => json!([version(
                    "L2v2l2v2",
                    "LIB2lib2",
                    "release",
                    json!([dep("LIB1lib1", Some("OTHERoth"))])
                )]),
                _ => json!([]),
            };
            parse_versions(&body, p, "forge", "1.12.2")
        };
        let err = plan_install(&target, "1.12.2", "forge", &no_installed, &versions_of)
            .unwrap_err()
            .to_string();
        assert!(err.contains("two different builds"), "{err}");
    }

    #[test]
    fn matching_pinned_build_is_accepted() {
        let dep = |p: &str, v: Option<&str>| json!({"project_id": p, "version_id": v, "dependency_type": "required"});
        let target = parse_versions(
            &json!([version(
                "TGT0tgt0",
                "ROOTroot",
                "release",
                json!([dep("LIB1lib1", Some("L1v1l1v1")), dep("LIB2lib2", None)])
            )]),
            "ROOTroot",
            "forge",
            "1.12.2",
        )
        .unwrap()
        .remove(0);
        let versions_of = |p: &str| -> Result<Vec<ModVersion>> {
            let body = match p {
                "LIB1lib1" => json!([version("L1v1l1v1", "LIB1lib1", "release", json!([]))]),
                "LIB2lib2" => json!([version(
                    "L2v2l2v2",
                    "LIB2lib2",
                    "release",
                    json!([dep("LIB1lib1", Some("L1v1l1v1"))])
                )]),
                _ => json!([]),
            };
            parse_versions(&body, p, "forge", "1.12.2")
        };
        let plan = plan_install(&target, "1.12.2", "forge", &no_installed, &versions_of).unwrap();
        assert_eq!(plan.installs.len(), 3, "agreeing pins must not conflict");
    }

    #[test]
    fn reverse_conflict_blocks_incoming_project() {
        let inc = |p: &str| Dependency {
            project_id: Some(p.into()),
            version_id: None,
            kind: DepKind::Incompatible,
        };
        let installed_deps = vec![("OLD0old0".to_string(), vec![inc("NEW0new0")])];
        assert!(reverse_conflict(&installed_deps, &["NEW0new0".to_string()]).is_err());
        assert!(reverse_conflict(&installed_deps, &["OTHERoth".to_string()]).is_ok());
    }

    #[test]
    fn search_url_carries_mod_and_loader_facets() {
        let u = search_url("forge", "sodium", 24);
        assert!(u.contains("project_type%3Amod"), "{u}");
        assert!(u.contains("categories%3Aforge"), "{u}");
        assert!(u.contains("query=sodium"), "{u}");
        let v = versions_url("AANobbMI", "legacy-fabric", "1.8.9");
        assert!(v.contains("legacy-fabric"), "{v}");
        assert!(v.contains("1.8.9"), "{v}");
    }

    #[test]
    fn parse_search_keeps_mods_only() {
        let body = json!({"hits": [
            {"project_id": "AAAAaaaa", "slug": "sodium", "title": "Sodium", "description": "d", "project_type": "mod", "downloads": 1},
            {"project_id": "BBBBbbbb", "slug": "pack", "title": "Pack", "description": "d", "project_type": "resourcepack", "downloads": 1}
        ], "offset": 0, "limit": 20, "total_hits": 2});
        let hits = parse_search(&body).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].slug, "sodium");
    }
    #[test]
    fn parse_search_accepts_real_world_titles() {
        // Live Modrinth titles carry slashes and non-ASCII text (display-only
        // strings; disk names come from filename_valid). Rejecting them used
        // to fail whole searches. Control characters stay unsafe.
        let body = json!({"hits": [
            {"project_id": "AAAAaaaa", "slug": "occlusion-fix", "title": "Sodium/Rubidium Occlusion Culling Fix", "project_type": "mod", "downloads": 1},
            {"project_id": "BBBBbbbb", "slug": "sodium-zh", "title": "钠-汉化模组（Sodium Chinese Localization）", "project_type": "mod", "downloads": 1}
        ]});
        assert_eq!(parse_search(&body).unwrap().len(), 2);
        let bad = json!({"hits": [{"project_id": "AAAAaaaa", "slug": "x", "title": "bad\u{0007}title", "project_type": "mod", "downloads": 1}]});
        assert!(parse_search(&bad).is_err());
    }
}
