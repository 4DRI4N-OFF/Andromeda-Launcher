//! Quilt profile validation. Shared strict Maven path parsing lives in fabric.
use serde_json::Value;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub fn validate_profile(p: &Value, game: &str, loader: &str) -> Result<()> {
    crate::fabric::maven_path(&format!("org.quiltmc:quilt-loader:{loader}"))?;
    crate::fabric::maven_path(&format!("org.quiltmc:hashed:{game}"))?;
    if p["inheritsFrom"].as_str() != Some(game)
        || p["id"].as_str() != Some(format!("quilt-loader-{loader}-{game}").as_str())
        || p["mainClass"].as_str() != Some("org.quiltmc.loader.impl.launch.knot.KnotClient")
    {
        return Err("Quilt profile does not match selection".into());
    }
    let libs = p["libraries"].as_array().ok_or("Missing Quilt libraries")?;
    if libs.is_empty() || libs.len() > 100 {
        return Err("Invalid Quilt library count".into());
    }
    let mut names = std::collections::HashSet::new();
    for l in libs {
        let n = l["name"].as_str().ok_or("Missing Quilt coordinate")?;
        crate::fabric::maven_path(n)?;
        let repo = l["url"].as_str().ok_or("Missing Quilt repository")?;
        if !names.insert(n)
            || !matches!(
                repo,
                "https://maven.fabricmc.net/" | "https://maven.quiltmc.org/repository/release/"
            )
        {
            return Err("Unapproved or duplicate Quilt library".into());
        }
    }
    for n in [
        format!("org.quiltmc:quilt-loader:{loader}"),
        format!("org.quiltmc:hashed:{game}"),
        format!("net.fabricmc:intermediary:{game}"),
    ] {
        if !names.contains(n.as_str()) {
            return Err("Required Quilt mapping/loader absent".into());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn p() -> Value {
        json!({"id":"quilt-loader-0.30.2-1.21.11","inheritsFrom":"1.21.11","mainClass":"org.quiltmc.loader.impl.launch.knot.KnotClient","libraries":[{"name":"org.quiltmc:quilt-loader:0.30.2","url":"https://maven.quiltmc.org/repository/release/"},{"name":"org.quiltmc:hashed:1.21.11","url":"https://maven.quiltmc.org/repository/release/"},{"name":"net.fabricmc:intermediary:1.21.11","url":"https://maven.fabricmc.net/"}]})
    }
    #[test]
    fn exact_profile() {
        assert!(validate_profile(&p(), "1.21.11", "0.30.2").is_ok());
    }
    #[test]
    fn wrong_game() {
        assert!(validate_profile(&p(), "1.21.10", "0.30.2").is_err());
    }
    #[test]
    fn hostile_repo() {
        let mut p = p();
        p["libraries"][0]["url"] = json!("https://example.invalid/");
        assert!(validate_profile(&p, "1.21.11", "0.30.2").is_err());
    }
    #[test]
    fn missing_mapping() {
        let mut p = p();
        p["libraries"].as_array_mut().unwrap().pop();
        assert!(validate_profile(&p, "1.21.11", "0.30.2").is_err());
    }
}
