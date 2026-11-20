#![cfg_attr(windows, windows_subsystem = "windows")]
//! Test-only offline launch, with no account access or token logging.
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() < 4 || a.len() > 7 {
        return Err(
            "Usage: loader_offline_probe VANILLA_ROOT GAME_DIR fabric|quilt|forge|neoforge [MC LOADER_VERSION] [launch]"
                .into(),
        );
    }
    let root = std::path::Path::new(&a[1]);
    let game = std::path::Path::new(&a[2]);
    // Default selection keeps the original probe behavior: the pinned
    // 1.21.11 builds. Any other combination is passed explicitly.
    let defaults = match a[3].as_str() {
        "fabric" => ("1.21.11", "0.20.1"),
        "quilt" => ("1.21.11", "0.30.2"),
        "forge" => ("1.21.11", "61.2.5"),
        "neoforge" => ("1.21.11", "21.11.48"),
        _ => return Err("Unsupported test loader".into()),
    };
    let (mc, loader_version, rest): (String, String, &[String]) = match a.get(4).map(String::as_str)
    {
        None | Some("launch") => (defaults.0.to_string(), defaults.1.to_string(), &a[4..]),
        Some(mc) => {
            let Some(lv) = a.get(5) else {
                return Err("MC version requires a loader version".into());
            };
            (mc.to_string(), lv.clone(), &a[6..])
        }
    };
    let launch = rest.first().map(String::as_str) == Some("launch");
    // Install or re-verify the requested vanilla version first; downloads
    // are hash-checked by the downloader.
    andromeda_core::vanilla::Downloader::new()?.install(root, &mc)?;
    let plan = match a[3].as_str() {
        "fabric" => andromeda_core::launch::prepare_fabric(
            root,
            &mc,
            "LoaderTest",
            2048,
            game,
            None,
            &loader_version,
        )?,
        "forge" => andromeda_core::launch::prepare_upstream(
            root,
            &mc,
            "LoaderTest",
            2048,
            game,
            None,
            &andromeda_core::upstream_install::Spec::new(
                andromeda_core::upstream_install::Kind::Forge,
                &mc,
                &loader_version,
            )?,
        )?,
        "neoforge" => andromeda_core::launch::prepare_upstream(
            root,
            &mc,
            "LoaderTest",
            2048,
            game,
            None,
            &andromeda_core::upstream_install::Spec::new(
                andromeda_core::upstream_install::Kind::NeoForge,
                &mc,
                &loader_version,
            )?,
        )?,
        "quilt" => andromeda_core::launch::prepare_quilt(
            root,
            &mc,
            "LoaderTest",
            2048,
            game,
            None,
            &loader_version,
        )?,
        _ => return Err("Unsupported test loader".into()),
    };
    if !plan
        .args
        .windows(2)
        .any(|v| v[0] == "--username" && v[1] == "LoaderTest")
        || !plan
            .args
            .windows(2)
            .any(|v| v[0] == "--accessToken" && v[1] == "0")
    {
        return Err("Offline identity not applied".into());
    }
    println!("Offline plan ready: {}", a[3]);
    if launch {
        let mut g = plan.spawn()?;
        std::fs::write(
            game.join("andromeda-probe.txt"),
            format!("Offline {} Java PID {}", a[3], g.id()),
        )?;
        let s = g.wait()?;
        std::fs::write(game.join("andromeda-probe-exit.txt"), format!("{s}"))?;
        if !s.success() {
            return Err("Game failed".into());
        }
    }
    Ok(())
}
