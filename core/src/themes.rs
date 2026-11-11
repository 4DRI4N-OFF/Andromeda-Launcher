//! Validated, versioned theme packs. Packs contain data, never executable code.
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemePack {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub description: String,
    pub dark: bool,
    pub colors: Colors,
    pub fonts: Fonts,
    pub background: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Colors {
    pub background: String,
    pub background_end: String,
    pub panel: String,
    pub panel_light: String,
    pub border: String,
    pub text: String,
    pub muted: String,
    pub accent: String,
    pub accent_end: String,
    pub on_accent: String,
    pub field: String,
    pub danger: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fonts {
    pub body: String,
    pub display: String,
    pub values: String,
}
/// Return RGBA bytes, accepting only #RRGGBB and #RRGGBBAA.
pub fn rgba(value: &str) -> Result<[u8; 4], String> {
    let hex = value.strip_prefix('#').ok_or("Color must start with #")?;
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("Invalid color: {value}"));
    }
    let mut result = [0, 0, 0, 255];
    for (i, byte) in result.iter_mut().enumerate().take(hex.len() / 2) {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string())?;
    }
    Ok(result)
}
impl ThemePack {
    pub fn parse(json: &str) -> Result<Self, String> {
        let pack: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if pack.schema_version != 1 {
            return Err("Unsupported theme schema".into());
        }
        if pack.id.is_empty()
            || pack.id.len() > 64
            || !pack
                .id
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        {
            return Err("Theme ID must be a short lowercase slug".into());
        }
        if pack.name.is_empty() || pack.name.len() > 80 || pack.description.len() > 240 {
            return Err("Theme name or description is invalid".into());
        }
        if !matches!(
            pack.background.as_str(),
            "galaxy" | "dawn" | "constellation"
        ) {
            return Err("Unknown background style".into());
        }
        for color in [
            &pack.colors.background,
            &pack.colors.background_end,
            &pack.colors.panel,
            &pack.colors.panel_light,
            &pack.colors.border,
            &pack.colors.text,
            &pack.colors.muted,
            &pack.colors.accent,
            &pack.colors.accent_end,
            &pack.colors.on_accent,
            &pack.colors.field,
            &pack.colors.danger,
        ] {
            rgba(color)?;
        }
        for font in [&pack.fonts.body, &pack.fonts.display, &pack.fonts.values] {
            if !matches!(
                font.as_str(),
                "Inter" | "Unbounded" | "Fraunces" | "Sora" | "JetBrains Mono"
            ) {
                return Err(format!("Font is not bundled: {font}"));
            }
        }
        Ok(pack)
    }
}
pub fn builtins() -> Vec<ThemePack> {
    [
        include_str!("../themes/horizonte.json"),
        include_str!("../themes/alba.json"),
        include_str!("../themes/cartografo.json"),
    ]
    .iter()
    .map(|json| ThemePack::parse(json).expect("bundled theme is valid"))
    .collect()
}
/// Invalid and oversized files are rejected individually, never crashing startup.
pub fn load_custom(directory: &Path) -> (Vec<ThemePack>, Vec<String>) {
    let mut packs = Vec::new();
    let mut errors = Vec::new();
    let Ok(entries) = fs::read_dir(directory) else {
        return (packs, errors);
    };
    let mut files: Vec<_> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    for path in files.into_iter().take(64) {
        let result: Result<ThemePack, String> = (|| {
            if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 32_768 {
                return Err("Theme exceeds 32 KiB".into());
            }
            let pack = ThemePack::parse(&fs::read_to_string(&path).map_err(|e| e.to_string())?)?;
            if builtins()
                .iter()
                .chain(packs.iter())
                .any(|p| p.id == pack.id)
            {
                return Err("Duplicate theme ID".into());
            }
            Ok(pack)
        })();
        match result {
            Ok(pack) => packs.push(pack),
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
    }
    (packs, errors)
}
/// Fixed motion recipe shared by every pack. Critically bypassed for reduced motion.
pub fn spring_scale(elapsed: f32, reduce_motion: bool) -> f32 {
    if reduce_motion || elapsed >= 0.5 {
        return 1.0;
    }
    1.0 + 0.04 * (-9.0 * elapsed).exp() * (elapsed * 25.0).sin()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_packs_are_valid_and_distinct() {
        let p = builtins();
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].id, "horizonte");
        assert!(!p[1].dark);
        assert_ne!(p[0].colors.accent, p[1].colors.accent);
    }
    #[test]
    fn invalid_values_are_rejected() {
        assert!(rgba("#ffzz00").is_err());
        assert!(rgba("red").is_err());
        assert_eq!(rgba("#ffffff80").unwrap(), [255, 255, 255, 128]);
        let json = include_str!("../themes/horizonte.json");
        for invalid in [
            json.replace("\"schema_version\": 1", "\"schema_version\": 99"),
            json.replace("Unbounded", "remote-font"),
            json.replace("horizonte", "../escape"),
        ] {
            assert!(ThemePack::parse(&invalid).is_err());
        }
    }
    #[test]
    fn motion_settles_and_reduced_motion_never_moves() {
        assert_eq!(spring_scale(0.0, false), 1.0);
        assert_eq!(spring_scale(0.5, false), 1.0);
        for i in 0..60 {
            let t = i as f32 / 100.0;
            assert_eq!(spring_scale(t, true), 1.0);
            assert!((0.96..=1.04).contains(&spring_scale(t, false)));
        }
    }
    #[test]
    fn custom_packs_do_not_override_builtins() {
        let dir = std::env::temp_dir().join(format!("andromeda-themes-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("duplicate.json"),
            include_str!("../themes/alba.json"),
        )
        .unwrap();
        fs::write(dir.join("invalid.json"), "{}").unwrap();
        fs::write(
            dir.join("custom.json"),
            include_str!("../themes/alba.json").replace("\"alba\"", "\"my-alba\""),
        )
        .unwrap();
        let (packs, errors) = load_custom(&dir);
        assert_eq!(packs.len(), 1);
        assert_eq!(errors.len(), 2);
        fs::remove_dir_all(dir).unwrap();
    }
}
