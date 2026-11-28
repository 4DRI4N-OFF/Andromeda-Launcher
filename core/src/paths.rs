//! Where runtime data lives.
//!
//! Debug builds keep data in the source workspace's `data/` folder so local
//! development matches the documented workflow. Release builds must never
//! write into a path compiled in on the build machine, so they use the
//! per-user platform directory instead. `ANDROMEDA_DATA_DIR` overrides both.
use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("ANDROMEDA_DATA_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir.trim());
        }
    }
    data_dir_default()
}

#[cfg(debug_assertions)]
fn data_dir_default() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|workspace| workspace.join("data"))
        .unwrap_or_else(|| PathBuf::from("data"))
}

#[cfg(not(debug_assertions))]
fn data_dir_default() -> PathBuf {
    platform_data_dir()
}

#[cfg(not(debug_assertions))]
fn platform_data_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(dir) = std::env::var("APPDATA") {
            if !dir.trim().is_empty() {
                return PathBuf::from(dir.trim()).join("Andromeda");
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(dir) = std::env::var("HOME") {
            if !dir.trim().is_empty() {
                return PathBuf::from(dir.trim()).join("Library/Application Support/Andromeda");
            }
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
            if !dir.trim().is_empty() {
                return PathBuf::from(dir.trim()).join("andromeda");
            }
        }
        if let Ok(dir) = std::env::var("HOME") {
            if !dir.trim().is_empty() {
                return PathBuf::from(dir.trim()).join(".local/share/andromeda");
            }
        }
    }
    PathBuf::from("data")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_env_override_wins_or_is_ignored_when_blank() {
        let custom = std::env::temp_dir().join("andromeda-data-override");
        std::env::set_var("ANDROMEDA_DATA_DIR", &custom);
        assert_eq!(data_dir(), custom);
        std::env::set_var("ANDROMEDA_DATA_DIR", "   ");
        let fallback = data_dir();
        assert!(!fallback.as_os_str().is_empty());
        assert_ne!(fallback, custom);
        std::env::remove_var("ANDROMEDA_DATA_DIR");
    }
}
