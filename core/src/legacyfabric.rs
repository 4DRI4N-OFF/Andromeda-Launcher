//! LegacyFabric metadata validation. No downloads or instance mutations yet.
//! LegacyFabric publishes standard fabric-loader builds for Minecraft 1.3
//! through 1.13.2 with its own intermediary, hosted on maven.legacyfabric.net.
use serde_json::Value;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
/// Hosts a LegacyFabric profile may point at: Fabric's own maven (ASM, mixin,
/// the loader itself) plus LegacyFabric's (intermediary, patched LWJGL).
fn approved_host(url: Option<&str>) -> bool {
    matches!(
        url,
        Some("https://maven.fabricmc.net/") | Some("https://maven.legacyfabric.net/")
    )
}
/// Validate a client overlay before it may affect any launch plan.
/// A later installer must verify each library digest before publishing it.
pub fn validate_profile(profile: &Value, game: &str, loader: &str) -> Result<()> {
    if super::fabric::maven_path(&format!("net.fabricmc:fabric-loader:{loader}")).is_err()
        || super::fabric::maven_path(&format!("net.legacyfabric:intermediary:{game}")).is_err()
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
        return Err("LegacyFabric profile does not match the selected game/loader".into());
    }
    let libraries = profile["libraries"]
        .as_array()
        .ok_or("Missing LegacyFabric libraries")?;
    if libraries.is_empty() || libraries.len() > 100 {
        return Err("Invalid LegacyFabric library count".into());
    }
    let mut names = std::collections::HashSet::new();
    for lib in libraries {
        let name = lib["name"].as_str().ok_or("Missing library coordinate")?;
        super::fabric::maven_path(name)?;
        if !names.insert(name) || !approved_host(lib["url"].as_str()) {
            return Err("Unapproved or duplicate LegacyFabric library".into());
        }
    }
    if !names.contains(format!("net.fabricmc:fabric-loader:{loader}").as_str())
        || !names.contains(format!("net.legacyfabric:intermediary:{game}").as_str())
    {
        return Err("Required LegacyFabric loader/intermediary missing".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fixture() -> Value {
        json!({"id":"fabric-loader-0.20.1-1.8.9","inheritsFrom":"1.8.9","mainClass":"net.fabricmc.loader.impl.launch.knot.KnotClient","libraries":[{"name":"org.ow2.asm:asm:9.10.1","url":"https://maven.fabricmc.net/"},{"name":"net.legacyfabric:intermediary:1.8.9","url":"https://maven.legacyfabric.net/"},{"name":"net.fabricmc:fabric-loader:0.20.1","url":"https://maven.fabricmc.net/"},{"name":"org.lwjgl.lwjgl:lwjgl:2.9.4+legacyfabric.17","url":"https://maven.legacyfabric.net/"}]})
    }
    #[test]
    fn client_overlay() {
        assert!(validate_profile(&fixture(), "1.8.9", "0.20.1").is_ok());
    }
    #[test]
    fn rejects_wrong_game() {
        assert!(validate_profile(&fixture(), "1.12.2", "0.20.1").is_err());
    }
    #[test]
    fn rejects_wrong_entrypoint() {
        let mut p = fixture();
        p["mainClass"] = json!("other.Main");
        assert!(validate_profile(&p, "1.8.9", "0.20.1").is_err());
    }
    #[test]
    fn rejects_unapproved_host() {
        let mut p = fixture();
        p["libraries"][0]["url"] = json!("https://example.invalid/");
        assert!(validate_profile(&p, "1.8.9", "0.20.1").is_err());
    }
    #[test]
    fn requires_both_libraries() {
        let mut p = fixture();
        p["libraries"]
            .as_array_mut()
            .unwrap()
            .retain(|l| l["name"].as_str() != Some("net.legacyfabric:intermediary:1.8.9"));
        assert!(validate_profile(&p, "1.8.9", "0.20.1").is_err());
    }
    #[test]
    fn accepts_fabric_loader_id_shape() {
        // Upstream reuses the "fabric-loader-{loader}-{game}" id shape.
        let mut p = fixture();
        p["id"] = json!("legacyfabric-1.8.9");
        assert!(validate_profile(&p, "1.8.9", "0.20.1").is_err());
    }
}
