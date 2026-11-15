#![cfg_attr(windows, windows_subsystem = "windows")]
//! Test-only offline launch, with no account access or token logging.
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() < 4 || a.len() > 5 {
        return Err("Usage: loader_offline_probe VANILLA_ROOT GAME_DIR fabric|quilt".into());
    }
    let root = std::path::Path::new(&a[1]);
    let game = std::path::Path::new(&a[2]);
    let plan = match a[3].as_str() {
        "fabric" => andromeda_core::launch::prepare_fabric(
            root,
            "1.21.11",
            "LoaderTest",
            2048,
            game,
            None,
            "0.20.1",
        )?,
        "forge" => andromeda_core::launch::prepare_upstream(
            root,
            "1.21.11",
            "LoaderTest",
            2048,
            game,
            None,
            andromeda_core::upstream_install::Kind::Forge,
        )?,
        "neoforge" => andromeda_core::launch::prepare_upstream(
            root,
            "1.21.11",
            "LoaderTest",
            2048,
            game,
            None,
            andromeda_core::upstream_install::Kind::NeoForge,
        )?,
        "quilt" => andromeda_core::launch::prepare_quilt(
            root,
            "1.21.11",
            "LoaderTest",
            2048,
            game,
            None,
            "0.30.2",
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
    if a.get(4).map(String::as_str) == Some("launch") {
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
