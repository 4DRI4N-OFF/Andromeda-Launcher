//! Local offline profile. Use an account/license you own; no online authentication.
use andromeda_core::launch;
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !(args.len() == 2 || args.len() == 3 && args[2] == "--dry-run") {
        return Err("Usage: andromeda-play 1.21.11 LOCAL_NAME [--dry-run]".into());
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("Missing workspace")?
        .join("data/vanilla");
    let plan = launch::prepare(&root, &args[0], &args[1], 2048)?;
    if args.len() == 3 {
        println!(
            "Offline launch ready: Java={}, game directory={}, {} arguments",
            plan.java.display(),
            plan.game_dir.display(),
            plan.args.len()
        );
        return Ok(());
    }
    println!(
        "Starting local profile {}. Online account authentication is not available in phase 2.",
        args[1]
    );
    let status = plan.spawn()?.wait()?;
    if !status.success() {
        return Err(format!("Minecraft exited with {status}; see game/launcher-logs").into());
    }
    Ok(())
}
