fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use andromeda_core::instances::{self, Loader};
    let root = std::path::Path::new("data/loader-regression-instances");
    let loader = Some(Loader::Fabric {
        version: "0.20.1".into(),
    });
    let a = instances::create_with_loader(
        root,
        "Fabric regression",
        "FabricTest",
        2048,
        loader.clone(),
    )?;
    assert_eq!(instances::get(root, &a.id)?.loader, loader);
    let b = instances::duplicate(root, &a.id, "Fabric duplicate")?;
    assert_eq!(b.loader, loader);
    instances::trash(root, &b.id)?;
    assert_eq!(instances::restore(root, &b.id)?.loader, loader);
    println!("Fabric create/read/duplicate/trash/restore preserved loader selection. No existing instances touched.");
    // Recoverable cleanup of only these newly created regression instances.
    instances::trash(root, &a.id)?;
    instances::trash(root, &b.id)?;
    Ok(())
}
