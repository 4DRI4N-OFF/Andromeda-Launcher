//! Babric metadata validation. No downloads or instance mutations yet.
//! Babric publishes standard fabric-loader builds for Minecraft b1.7.3 with
//! its own intermediary, hosted on maven.glass-launcher.net.
use serde_json::Value;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
/// Hosts a Babric profile may point at: Fabric's own maven (ASM, mixin, the
/// loader itself), Glass Launcher's (intermediary, patched LWJGL, log4j
/// config), Maven Central (terminalconsoleappender) and Mojang's libraries
/// host (logging and support jars).
fn approved_host(url: Option<&str>) -> bool {
    matches!(
        url,
        Some("https://maven.fabricmc.net/")
            | Some("https://maven.glass-launcher.net/babric/")
            | Some("https://repo1.maven.org/maven2/")
            | Some("https://libraries.minecraft.net/")
    )
}
/// Validate a client overlay before it may affect any launch plan.
/// A later installer must verify each library digest before publishing it.
pub fn validate_profile(profile: &Value, game: &str, loader: &str) -> Result<()> {
    if super::fabric::maven_path(&format!("net.fabricmc:fabric-loader:{loader}")).is_err()
        || super::fabric::maven_path(&format!("babric:intermediary-upstream:{game}")).is_err()
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
        return Err("Babric profile does not match the selected game/loader".into());
    }
    let libraries = profile["libraries"]
        .as_array()
        .ok_or("Missing Babric libraries")?;
    if libraries.is_empty() || libraries.len() > 100 {
        return Err("Invalid Babric library count".into());
    }
    let mut names = std::collections::HashSet::new();
    for lib in libraries {
        let name = lib["name"].as_str().ok_or("Missing library coordinate")?;
        super::fabric::maven_path(name)?;
        if !names.insert(name) || !approved_host(lib["url"].as_str()) {
            return Err("Unapproved or duplicate Babric library".into());
        }
    }
    if !names.contains(format!("net.fabricmc:fabric-loader:{loader}").as_str())
        || !names.contains(format!("babric:intermediary-upstream:{game}").as_str())
    {
        return Err("Required Babric loader/intermediary missing".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fixture() -> Value {
        json!({"id":"fabric-loader-0.20.1-b1.7.3","inheritsFrom":"b1.7.3","mainClass":"net.fabricmc.loader.impl.launch.knot.KnotClient","libraries":[{"name":"org.ow2.asm:asm:9.10.1","url":"https://maven.fabricmc.net/"},{"name":"babric:intermediary-upstream:b1.7.3","url":"https://maven.glass-launcher.net/babric/"},{"name":"net.fabricmc:fabric-loader:0.20.1","url":"https://maven.fabricmc.net/"},{"name":"org.lwjgl.lwjgl:lwjgl:2.9.4-babric.1","url":"https://maven.glass-launcher.net/babric/"}]})
    }
    #[test]
    fn client_overlay() {
        assert!(validate_profile(&fixture(), "b1.7.3", "0.20.1").is_ok());
    }
    #[test]
    fn rejects_wrong_game() {
        assert!(validate_profile(&fixture(), "1.12.2", "0.20.1").is_err());
    }
    #[test]
    fn rejects_wrong_entrypoint() {
        let mut p = fixture();
        p["mainClass"] = json!("other.Main");
        assert!(validate_profile(&p, "b1.7.3", "0.20.1").is_err());
    }
    #[test]
    fn rejects_unapproved_host() {
        let mut p = fixture();
        p["libraries"][0]["url"] = json!("https://example.invalid/");
        assert!(validate_profile(&p, "b1.7.3", "0.20.1").is_err());
    }
    #[test]
    fn requires_both_libraries() {
        let mut p = fixture();
        p["libraries"]
            .as_array_mut()
            .unwrap()
            .retain(|l| l["name"].as_str() != Some("babric:intermediary-upstream:b1.7.3"));
        assert!(validate_profile(&p, "b1.7.3", "0.20.1").is_err());
    }
    #[test]
    fn accepts_fabric_loader_id_shape() {
        // Upstream reuses the "fabric-loader-{loader}-{game}" id shape.
        let mut p = fixture();
        p["id"] = json!("babric-b1.7.3");
        assert!(validate_profile(&p, "b1.7.3", "0.20.1").is_err());
    }
}
