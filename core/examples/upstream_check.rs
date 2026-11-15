#![cfg_attr(windows, windows_subsystem = "windows")]
//! Installer-only probe. Does not open Minecraft or access account sessions.
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return;
    }
    let kind = match args[1].as_str() {
        "forge" => andromeda_core::upstream_install::Kind::Forge,
        "neoforge" => andromeda_core::upstream_install::Kind::NeoForge,
        _ => return,
    };
    let root = std::path::PathBuf::from(format!("data/{}-installation-probe", args[1]));
    let work = || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::fs::create_dir_all(&root)?;
        std::fs::write(root.join("probe-status.txt"), "Installing")?;
        let vanilla = std::path::Path::new("data/vanilla");
        let plan = andromeda_core::launch::prepare(vanilla, "1.21.11", "LoaderTest", 2048)?;
        let installed =
            andromeda_core::upstream_install::install(&root, &plan.java, vanilla, kind)?;
        std::fs::write(
            root.join("probe-status.txt"),
            format!(
                "Verified {} launch libraries for {}",
                installed.libraries.len(),
                kind.version()
            ),
        )?;
        Ok(())
    };
    if let Err(e) = work() {
        let _ = std::fs::write(root.join("probe-status.txt"), format!("Failed: {e}"));
    }
}
