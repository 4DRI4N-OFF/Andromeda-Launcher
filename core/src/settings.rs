use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub reduce_animations: bool,
    pub dark_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            reduce_animations: false,
            dark_mode: true,
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
        // Write beside the target, flush, then rename: a crash mid-write can
        // never truncate the previous settings file.
        let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
        let mut file = fs::File::create(&tmp)?;
        io::Write::write_all(&mut file, json.as_bytes())?;
        file.sync_all()?;
        drop(file);
        match fs::rename(&tmp, path) {
            Ok(()) => Ok(()),
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                Err(e)
            }
        }
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
    fn roundtrip() {
        let dir = std::env::temp_dir().join("andromeda-test-settings");
        let p = dir.join("settings.json");
        let s = Settings {
            reduce_animations: true,
            dark_mode: false,
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

    #[test]
    fn save_replaces_existing_file_and_leaves_no_temp_behind() {
        let dir = std::env::temp_dir().join("andromeda-test-atomic");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("settings.json");
        fs::write(&p, "{\"corrupt\": true").unwrap();
        let s = Settings {
            reduce_animations: true,
            dark_mode: true,
        };
        s.save(&p).unwrap();
        assert_eq!(Settings::load(&p), s);
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
        let _ = fs::remove_dir_all(dir);
    }
}
