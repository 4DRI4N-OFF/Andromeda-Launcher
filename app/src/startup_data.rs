//! Local, motion-independent startup reads. Call on a worker after showing splash.
//! No authentication refresh, network, game launch, or file writes occur here.
use andromeda_core::{instances::Instance,settings::Settings,themes::ThemePack};
use std::{path::Path,sync::atomic::{AtomicBool,Ordering}};
pub struct Prepared {
    pub settings: Settings,
    pub themes: Vec<ThemePack>,
    pub instances: Vec<Instance>,
    pub warnings: Vec<String>,
    pub has_saved_session: bool,
}
#[derive(Debug,PartialEq,Eq)]
pub enum PrepareError { Cancelled }
pub fn prepare(root:&Path,cancel:&AtomicBool)->Result<Prepared,PrepareError> {
    let check=|| if cancel.load(Ordering::Relaxed) {Err(PrepareError::Cancelled)} else {Ok(())};
    check()?;
    // Preserve existing missing/corrupt settings fallback. Never overwrite source.
    let settings=Settings::load(&root.join("settings.json"));
    check()?;
    let mut themes=andromeda_core::themes::builtins();
    let (custom,mut warnings)=andromeda_core::themes::load_custom(&root.join("themes"));
    themes.extend(custom);
    check()?;
    // Match the launcher's existing recoverable instance-list error behavior.
    let instances=match andromeda_core::instances::list(&root.join("instances")) {
        Ok(items)=>items,
        Err(error)=>{warnings.push(format!("Could not read instances: {error}"));Vec::new()}
    };
    check()?;
    // Only inspect presence. Protected tokens remain in their store and refresh
    // remains an independent background operation, never a gate on opening UI.
    let has_saved_session=root.join("auth/session.dpapi").is_file();
    check()?;
    Ok(Prepared {settings,themes,instances,warnings,has_saved_session})
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs,time::{SystemTime,UNIX_EPOCH}};
    struct Fixture(std::path::PathBuf);
    impl Fixture {fn new()->Self {let p=std::env::temp_dir().join(format!("andromeda-startup-data-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir_all(&p).unwrap();Self(p)}}
    impl Drop for Fixture {fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
    #[test] fn no_data_is_valid_without_creating_files(){let f=Fixture::new();let p=prepare(&f.0,&AtomicBool::new(false)).unwrap();assert_eq!(p.settings,Settings::default());assert_eq!(p.themes.len(),3);assert!(p.instances.is_empty());assert!(p.warnings.is_empty());assert!(!p.has_saved_session);assert_eq!(fs::read_dir(&f.0).unwrap().count(),0);}
    #[test] fn cancellation_is_not_readiness(){let f=Fixture::new();assert!(matches!(prepare(&f.0,&AtomicBool::new(true)),Err(PrepareError::Cancelled)));}
    #[test] fn corrupt_settings_are_preserved(){let f=Fixture::new();let p=f.0.join("settings.json");fs::write(&p,b"broken").unwrap();let result=prepare(&f.0,&AtomicBool::new(false)).unwrap();assert_eq!(result.settings,Settings::default());assert_eq!(fs::read(&p).unwrap(),b"broken");}
    #[test] fn invalid_theme_warns_without_hiding_builtins(){let f=Fixture::new();fs::create_dir(f.0.join("themes")).unwrap();fs::write(f.0.join("themes/bad.json"),b"broken").unwrap();let p=prepare(&f.0,&AtomicBool::new(false)).unwrap();assert_eq!(p.themes.len(),3);assert_eq!(p.warnings.len(),1);}
    #[test] fn invalid_instance_is_recoverable_and_untouched(){let f=Fixture::new();fs::create_dir(f.0.join("instances")).unwrap();fs::write(f.0.join("instances/invalid"),b"keep").unwrap();let p=prepare(&f.0,&AtomicBool::new(false)).unwrap();assert!(p.instances.is_empty());assert_eq!(p.warnings.len(),1);assert_eq!(fs::read(f.0.join("instances/invalid")).unwrap(),b"keep");}
    #[test] fn session_presence_does_not_read_or_refresh_tokens(){let f=Fixture::new();fs::create_dir(f.0.join("auth")).unwrap();let path=f.0.join("auth/session.dpapi");fs::write(&path,b"fixture-not-a-real-session").unwrap();let p=prepare(&f.0,&AtomicBool::new(false)).unwrap();assert!(p.has_saved_session);assert_eq!(fs::read(path).unwrap(),b"fixture-not-a-real-session");}
}
