#![cfg_attr(windows, windows_subsystem = "windows")]
//! Test-only offline launch, with no account access or token logging.
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() < 4 || a.len() > 7 {
        return Err(
            "Usage: loader_offline_probe VANILLA_ROOT GAME_DIR fabric|quilt|forge|neoforge|legacyfabric|babric|vanilla-legacy [MC LOADER_VERSION] [launch]"
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
        "legacyfabric" => ("1.8.9", "0.20.1"),
        "babric" => ("b1.7.3", "0.20.1"),
        "vanilla-legacy" => ("1.5.2", ""),
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
        "vanilla-legacy" => {
            andromeda_core::launch::prepare_in_game_dir(root, &mc, "LoaderTest", 2048, game)?
        }
        "babric" => andromeda_core::launch::prepare_babric(
            root,
            &mc,
            "LoaderTest",
            2048,
            game,
            None,
            &loader_version,
        )?,
        "legacyfabric" => andromeda_core::launch::prepare_legacyfabric(
            root,
            &mc,
            "LoaderTest",
            2048,
            game,
            None,
            &loader_version,
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
    if matches!(a[3].as_str(), "vanilla-legacy" | "babric") {
        // Pre-1.6 identity is positional: the two game arguments right
        // after the main class are player name then session token. A loader
        // profile swaps the main class for its own entrypoint (KnotClient).
        let main = plan
            .args
            .iter()
            .position(|x| x.starts_with("net.minecraft.") || x.ends_with("KnotClient"))
            .ok_or("Legacy main class missing from plan")?;
        if plan.args.get(main + 1).map(String::as_str) != Some("LoaderTest")
            || plan.args.get(main + 2).map(String::as_str) != Some("0")
        {
            return Err("Legacy positional identity not applied".into());
        }
        if plan.args.iter().any(|x| x.contains("${")) {
            return Err("Unexpanded placeholder in legacy arguments".into());
        }
        let virtual_assets = root.join("assets").join("virtual").join("pre-1.6");
        let count = std::fs::read_dir(&virtual_assets)
            .map_err(|_| "Virtual legacy assets not assembled".to_owned())?
            .count();
        if count == 0 {
            return Err("Virtual legacy assets are empty".into());
        }
        println!("Legacy positional plan verified: {count} top-level virtual asset entries");
    } else if !plan
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
    if a[3].as_str() == "legacyfabric" {
        if !plan.java.to_string_lossy().contains("jre-legacy") {
            return Err("Legacy launch must use the Mojang legacy Java runtime".into());
        }
        let cp = plan
            .args
            .windows(2)
            .find(|v| v[0] == "-cp")
            .map(|v| v[1].clone())
            .unwrap_or_default();
        for needle in ["fabric-loader-0.20.1", "intermediary-1.8.9", "1.8.9.jar"] {
            if !cp.contains(needle) {
                return Err(format!("Legacy classpath missing {needle}").into());
            }
        }
        if plan.args.iter().any(|x| x.contains("${")) {
            return Err("Unexpanded placeholder in legacy arguments".into());
        }
        let ndir = root
            .join("natives")
            .join(format!("{mc}-legacyfabric-{loader_version}"));
        if !ndir.join("lwjgl64.dll").is_file() {
            return Err("Legacy natives not extracted".into());
        }
        println!("Legacy plan verified: jre-legacy runtime, classpath, arguments, natives");
    }
    if a[3].as_str() == "babric" {
        if !plan.java.to_string_lossy().contains("jre-legacy") {
            return Err("Babric launch must use the Mojang legacy Java runtime".into());
        }
        let cp = plan
            .args
            .windows(2)
            .find(|v| v[0] == "-cp")
            .map(|v| v[1].clone())
            .unwrap_or_default();
        for needle in [
            "fabric-loader-0.20.1",
            "intermediary-upstream-b1.7.3",
            "b1.7.3.jar",
        ] {
            if !cp.contains(needle) {
                return Err(format!("Babric classpath missing {needle}").into());
            }
        }
        if plan.args.iter().any(|x| x.contains("${")) {
            return Err("Unexpanded placeholder in Babric arguments".into());
        }
        let ndir = root
            .join("natives")
            .join(format!("{mc}-babric-{loader_version}"));
        if !ndir.join("lwjgl64.dll").is_file() {
            return Err("Babric natives not extracted".into());
        }
        println!("Babric plan verified: jre-legacy runtime, classpath, arguments, natives");
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
