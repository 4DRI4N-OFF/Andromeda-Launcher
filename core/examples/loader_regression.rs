//! Recoverable regression checks on fresh test-only instances. Never touches an existing instance.
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use andromeda_core::instances::{self, Loader};
    let root = std::path::Path::new("data/loader-regression-instances");
    for loader in [
        Loader::Fabric {
            version: "0.20.1".into(),
        },
        Loader::Quilt {
            version: "0.30.2".into(),
        },
        Loader::Forge {
            version: "61.2.5".into(),
        },
        Loader::NeoForge {
            version: "21.11.48".into(),
        },
    ] {
        let a = instances::create_with_loader(
            root,
            "Loader regression",
            "LoaderTest",
            2048,
            Some(loader.clone()),
        )?;
        assert_eq!(instances::get(root, &a.id)?.loader, Some(loader.clone()));
        let b = instances::duplicate(root, &a.id, "Loader duplicate")?;
        assert_eq!(b.loader, Some(loader.clone()));
        instances::trash(root, &b.id)?;
        assert_eq!(
            instances::restore(root, &b.id)?.loader,
            Some(loader.clone())
        );
        println!("Create/read/duplicate/trash/restore preserved {loader:?}");
        instances::trash(root, &a.id)?;
        instances::trash(root, &b.id)?;
    }
    Ok(())
}
