//! Fabric metadata validation. No downloads or instance mutations yet.
use serde_json::Value;
use std::path::PathBuf;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn component(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s != "."
        && s != ".."
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-+".contains(&b))
}
/// Convert strict Maven coordinates to a relative path, never an absolute path.
pub fn maven_path(coordinate: &str) -> Result<PathBuf> {
    let parts: Vec<_> = coordinate.split(':').collect();
    if parts.len() != 3 || !parts.iter().all(|p| component(p)) {
        return Err("Unsupported or unsafe Maven coordinate".into());
    }
    let mut path = PathBuf::new();
    for part in parts[0].split('.') {
        if !component(part) {
            return Err("Invalid Maven group".into());
        }
        path.push(part);
    }
    path.push(parts[1]);
    path.push(parts[2]);
    path.push(format!("{}-{}.jar", parts[1], parts[2]));
    Ok(path)
}
/// Validate a client overlay before it may affect any launch plan.
/// A later installer must verify each library digest before publishing it.
pub fn validate_profile(profile: &Value, game: &str, loader: &str) -> Result<()> {
    if !component(game)
        || !component(loader)
        || profile["inheritsFrom"].as_str() != Some(game)
        || profile["id"].as_str() != Some(format!("fabric-loader-{loader}-{game}").as_str())
        || !matches!(
            profile["mainClass"].as_str(),
            Some(
                "net.fabricmc.loader.impl.launch.knot.KnotClient"
                    | "net.fabricmc.loader.launch.knot.KnotClient"
            )
        )
    {
        return Err("Fabric profile does not match the selected game/loader".into());
    }
    let libraries = profile["libraries"]
        .as_array()
        .ok_or("Missing Fabric libraries")?;
    if libraries.is_empty() || libraries.len() > 100 {
        return Err("Invalid Fabric library count".into());
    }
    let mut names = std::collections::HashSet::new();
    for lib in libraries {
        let name = lib["name"].as_str().ok_or("Missing library coordinate")?;
        maven_path(name)?;
        if !names.insert(name) || lib["url"].as_str() != Some("https://maven.fabricmc.net/") {
            return Err("Unapproved or duplicate Fabric library".into());
        }
    }
    if !names.contains(format!("net.fabricmc:fabric-loader:{loader}").as_str())
        || !names.contains(format!("net.fabricmc:intermediary:{game}").as_str())
    {
        return Err("Required Fabric loader/intermediary missing".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fixture() -> Value {
        json!({"id":"fabric-loader-0.20.1-1.21.11","inheritsFrom":"1.21.11","mainClass":"net.fabricmc.loader.impl.launch.knot.KnotClient","libraries":[{"name":"net.fabricmc:fabric-loader:0.20.1","url":"https://maven.fabricmc.net/"},{"name":"net.fabricmc:intermediary:1.21.11","url":"https://maven.fabricmc.net/"}]})
    }
    #[test]
    fn maven_layout() {
        assert_eq!(
            maven_path("org.ow2.asm:asm:9.10.1").unwrap(),
            PathBuf::from("org/ow2/asm/asm/9.10.1/asm-9.10.1.jar")
        );
    }
    #[test]
    fn rejects_traversal() {
        for s in [
            "../x:a:1",
            "net..x:a:1",
            "net:x:../1",
            "net:x:1:classifier",
            "C:/x:a:1",
            "net:x:1?x",
        ] {
            assert!(maven_path(s).is_err(), "{s}");
        }
    }
    #[test]
    fn client_overlay() {
        assert!(validate_profile(&fixture(), "1.21.11", "0.20.1").is_ok());
    }
    #[test]
    fn rejects_wrong_game() {
        assert!(validate_profile(&fixture(), "1.21.10", "0.20.1").is_err());
    }
    #[test]
    fn rejects_wrong_entrypoint() {
        let mut p = fixture();
        p["mainClass"] = json!("other.Main");
        assert!(validate_profile(&p, "1.21.11", "0.20.1").is_err());
    }
    #[test]
    fn rejects_unapproved_host() {
        let mut p = fixture();
        p["libraries"][0]["url"] = json!("https://example.invalid/");
        assert!(validate_profile(&p, "1.21.11", "0.20.1").is_err());
    }
    #[test]
    fn requires_both_libraries() {
        let mut p = fixture();
        p["libraries"].as_array_mut().unwrap().pop();
        assert!(validate_profile(&p, "1.21.11", "0.20.1").is_err());
    }
}
