use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub reduce_animations: bool,
    pub dark_mode: bool,
    pub theme_id: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            reduce_animations: false,
            dark_mode: true,
            theme_id: "horizonte".into(),
        }
    }
}

impl Settings {
    /// Missing or corrupt file gives defaults, never a crash.
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_missing() {
        let s = Settings::load(Path::new("no/existe/settings.json"));
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn legacy_settings_keep_motion_and_gain_theme() {
        let s: Settings =
            serde_json::from_str(r#"{"reduce_animations":true,"dark_mode":false}"#).unwrap();
        assert!(s.reduce_animations);
        assert_eq!(s.theme_id, "horizonte");
    }

    #[test]
    fn roundtrip() {
        let dir = std::env::temp_dir().join("andromeda-test-settings");
        let p = dir.join("settings.json");
        let s = Settings {
            reduce_animations: true,
            dark_mode: false,
            theme_id: "alba".into(),
        };
        s.save(&p).unwrap();
        assert_eq!(Settings::load(&p), s);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn corrupt_file_gives_defaults() {
        let dir = std::env::temp_dir().join("andromeda-test-corrupt");
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("settings.json");
        fs::write(&p, "{ esto no es json").unwrap();
        assert_eq!(Settings::load(&p), Settings::default());
        let _ = fs::remove_dir_all(dir);
    }
}
