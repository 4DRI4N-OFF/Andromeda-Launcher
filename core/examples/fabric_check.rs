//! Headless phase-5 validation; installs no mods and launches no game.
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("Usage: fabric_check CACHE_DIR GAME_VERSION LOADER_VERSION".into());
    }
    let installed = andromeda_core::fabric_install::install(
        std::path::Path::new(&args[1]),
        &args[2],
        &args[3],
    )?;
    println!(
        "Verified {} Fabric client libraries for {} with loader {}",
        installed.libraries.len(),
        args[2],
        args[3]
    );
    println!(
        "Client entrypoint: {}",
        installed.profile["mainClass"]
            .as_str()
            .ok_or("Missing main class")?
    );
    Ok(())
}
