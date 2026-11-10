//! Builds a launch plan only. Does not start Java or read an account session.
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = std::path::Path::new("data/vanilla");
    let game = std::path::Path::new("data/fabric-probe-game");
    let plan = andromeda_core::launch::prepare_fabric(
        root,
        "1.21.11",
        "FabricTest",
        2048,
        game,
        None,
        "0.20.1",
    )?;
    let entry = plan
        .args
        .iter()
        .position(|a| a == "net.fabricmc.loader.impl.launch.knot.KnotClient")
        .ok_or("Fabric entrypoint absent")?;
    let cp = plan
        .args
        .windows(2)
        .find(|a| a[0] == "-cp")
        .ok_or("Classpath absent")?;
    if !cp[1].contains("fabric-loader") || !cp[1].contains("intermediary") {
        return Err("Fabric libraries absent from classpath".into());
    }
    if plan.args[entry + 1..].iter().all(|a| a != "--username") {
        return Err("Inherited vanilla game arguments absent".into());
    }
    if !plan.args[..entry]
        .iter()
        .any(|a| a.starts_with("-DFabricMcEmu="))
    {
        return Err("Fabric JVM arguments absent".into());
    }
    println!("Verified Fabric launch plan: client entrypoint, loader/intermediary classpath, inherited vanilla game args and Fabric JVM overlay. Java was not launched.");
    Ok(())
}
