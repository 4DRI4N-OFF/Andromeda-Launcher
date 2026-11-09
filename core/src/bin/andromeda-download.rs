use andromeda_core::vanilla::Downloader;
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let d = Downloader::new()?;
    match args.first().map(String::as_str) {
        Some("list") => {
            let v = d.versions()?;
            for entry in v["versions"].as_array().ok_or("Missing versions")? {
                if entry["type"] == "release" {
                    println!("{}", entry["id"].as_str().unwrap_or("?"));
                }
            }
        }
        Some("install") if args.len() == 2 => {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .ok_or("Missing workspace root")?
                .join("data/vanilla");
            println!(
                "{}",
                serde_json::to_string_pretty(&d.install(&root, &args[1])?)?
            );
        }
        _ => return Err(
            "Usage: andromeda-download list | install VERSION. Windows x64 only; no game launch."
                .into(),
        ),
    }
    Ok(())
}
