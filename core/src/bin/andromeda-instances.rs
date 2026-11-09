//! Manage isolated local instances without touching the default game folder.
use andromeda_core::{instances, launch};
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("Missing workspace")?
        .join("data");
    let root = data.join("instances");
    match args.first().map(String::as_str) {
        Some("list") if args.len()==1 => println!("{}", serde_json::to_string_pretty(&instances::list(&root)?)?),
        Some("create") if args.len()==3 => println!("{}", serde_json::to_string_pretty(&instances::create(&root,&args[1],&args[2],2048)?)?),
        Some("duplicate") if args.len()==3 => println!("{}", serde_json::to_string_pretty(&instances::duplicate(&root,&args[1],&args[2])?)?),
        Some("trash") if args.len()==2 => println!("Moved to recoverable launcher trash: {}", instances::trash(&root,&args[1])?.display()),
        Some("restore") if args.len()==2 => println!("{}", serde_json::to_string_pretty(&instances::restore(&root,&args[1])?)?),
        Some("play") if args.len()==2 || args.len()==3 && args[2]=="--dry-run" => {
            let i = instances::get(&root,&args[1])?;
            let game = instances::game_dir(&root,&i.id)?;
            let plan=launch::prepare_in_game_dir(&data.join("vanilla"),&i.version,&i.local_name,i.memory_mb,&game)?;
            if args.len()==3 { println!("Ready: {} ({})", i.name,plan.game_dir.display()); }
            else if !plan.spawn()?.wait()?.success() { return Err("Game exited unsuccessfully; inspect this instance's launcher-logs".into()); }
        }
        _ => return Err("Usage: andromeda-instances list | create NAME LOCAL_NAME | duplicate ID NAME | trash ID | restore ID | play ID [--dry-run]".into())
    }
    Ok(())
}
