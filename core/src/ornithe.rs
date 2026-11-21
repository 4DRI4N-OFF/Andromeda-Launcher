//! Ornithe metadata validation. No downloads or instance mutations yet.
//! Ornithe repackages fabric-loader as net.ornithemc:ornithe-loader for
//! Minecraft b1.7.3 through 1.13.2 with its own calamus intermediary,
//! hosted on maven.ornithemc.net.
use serde_json::Value;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
/// Hosts an Ornithe profile may point at: Ornithe's own maven (loader,
/// calamus, nester) plus Fabric's (ASM, mixin, tiny tools).
fn approved_host(url: Option<&str>) -> bool {
    matches!(
        url,
        Some("https://maven.ornithemc.net/releases/") | Some("https://maven.fabricmc.net/")
    )
}
/// Extract and validate the calamus intermediary coordinate from a
/// /v2/versions/loader/{game}/{loader} detail entry. Client metadata only:
/// the server environment is out of scope for this launcher.
pub fn calamus_version(detail: &Value, game: &str) -> Result<String> {
    let calamus = &detail["calamus"];
    let version = calamus["version"]
        .as_str()
        .ok_or("Missing Ornithe calamus version")?;
    if calamus["maven"].as_str() != Some(format!("net.ornithemc:calamus:{version}").as_str())
        || calamus["gameVersion"].as_str() != Some(game)
        || calamus["environment"].as_str() != Some("client")
        || super::fabric::maven_path(&format!("net.ornithemc:calamus:{version}")).is_err()
    {
        return Err("Ornithe calamus entry does not match the selected game".into());
    }
    Ok(version.to_owned())
}
/// Validate a client overlay before it may affect any launch plan.
/// A later installer must verify each library digest before publishing it.
pub fn validate_profile(profile: &Value, _game: &str, loader: &str, calamus: &str) -> Result<()> {
    // The game binding is validated on the calamus entry itself
    // (calamus_version checks gameVersion); the profile matches calamus.
    if super::fabric::maven_path(&format!("net.ornithemc:ornithe-loader:{loader}")).is_err()
        || profile["inheritsFrom"].as_str() != Some(calamus)
        || profile["id"].as_str() != Some(format!("ornithe-loader-{loader}-{calamus}").as_str())
        || !matches!(
            profile["mainClass"].as_str(),
            Some(
                "net.ornithemc.loader.impl.launch.knot.KnotClient"
                    | "net.ornithemc.loader.launch.knot.KnotClient"
            )
        )
    {
        return Err("Ornithe profile does not match the selected game/loader".into());
    }
    let libraries = profile["libraries"]
        .as_array()
        .ok_or("Missing Ornithe libraries")?;
    if libraries.is_empty() || libraries.len() > 100 {
        return Err("Invalid Ornithe library count".into());
    }
    let mut names = std::collections::HashSet::new();
    for lib in libraries {
        let name = lib["name"].as_str().ok_or("Missing library coordinate")?;
        super::fabric::maven_path(name)?;
        if !names.insert(name) || !approved_host(lib["url"].as_str()) {
            return Err("Unapproved or duplicate Ornithe library".into());
        }
    }
    if !names.contains(format!("net.ornithemc:ornithe-loader:{loader}").as_str())
        || !names.contains(format!("net.ornithemc:calamus:{calamus}").as_str())
    {
        return Err("Required Ornithe loader/intermediary missing".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn detail() -> Value {
        json!({"calamus":{"version":"b1.7.3-client","stable":true,"maven":"net.ornithemc:calamus:b1.7.3-client","gameVersion":"b1.7.3","environment":"client"}})
    }
    fn fixture() -> Value {
        json!({"id":"ornithe-loader-0.1.2-b1.7.3-client","inheritsFrom":"b1.7.3-client","mainClass":"net.ornithemc.loader.impl.launch.knot.KnotClient","libraries":[{"name":"org.ow2.asm:asm:9.3","url":"https://maven.fabricmc.net/"},{"name":"net.ornithemc:calamus:b1.7.3-client","url":"https://maven.ornithemc.net/releases/"},{"name":"net.ornithemc:ornithe-loader:0.1.2","url":"https://maven.ornithemc.net/releases/"}]})
    }
    #[test]
    fn calamus_entry() {
        assert_eq!(
            calamus_version(&detail(), "b1.7.3").unwrap(),
            "b1.7.3-client"
        );
        assert!(calamus_version(&detail(), "1.8.9").is_err());
    }
    #[test]
    fn client_overlay() {
        assert!(validate_profile(&fixture(), "b1.7.3", "0.1.2", "b1.7.3-client").is_ok());
    }
    #[test]
    fn rejects_unsuffixed_inherits() {
        // The base id must come from the calamus coordinate, not the game id:
        // old Ornithe versions inherit a "-client" environment id.
        assert!(validate_profile(&fixture(), "b1.7.3", "0.1.2", "b1.7.3").is_err());
    }
    #[test]
    fn rejects_wrong_entrypoint() {
        let mut p = fixture();
        p["mainClass"] = json!("other.Main");
        assert!(validate_profile(&p, "b1.7.3", "0.1.2", "b1.7.3-client").is_err());
    }
    #[test]
    fn rejects_unapproved_host() {
        let mut p = fixture();
        p["libraries"][0]["url"] = json!("https://example.invalid/");
        assert!(validate_profile(&p, "b1.7.3", "0.1.2", "b1.7.3-client").is_err());
    }
    #[test]
    fn requires_both_libraries() {
        let mut p = fixture();
        p["libraries"]
            .as_array_mut()
            .unwrap()
            .retain(|l| l["name"].as_str() != Some("net.ornithemc:calamus:b1.7.3-client"));
        assert!(validate_profile(&p, "b1.7.3", "0.1.2", "b1.7.3-client").is_err());
    }
}
