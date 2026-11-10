fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() != 4 {
        return Err("Usage: quilt_check CACHE GAME LOADER".into());
    }
    let i = andromeda_core::quilt_install::install(std::path::Path::new(&a[1]), &a[2], &a[3])?;
    println!(
        "Verified {} Quilt client libraries for {} / {}",
        i.libraries.len(),
        a[2],
        a[3]
    );
    Ok(())
}
