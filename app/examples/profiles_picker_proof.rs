//! Profiles picker proof with LIVE catalog data. Renders settled states
//! (animations reduced) of each version filter, proves the refresh callback
//! changes the listed data, shows loader availability for a loader-less
//! version and the loader build list for a picked loader. No window opened.
use andromeda_core::catalog::{available_loaders, loader_versions, mc_versions, LoaderKind, McFilter, McKind};
use slint::{platform::{software_renderer::{MinimalSoftwareWindow,RepaintBufferType},Platform,WindowAdapter},ComponentHandle,Model,Rgb8Pixel,ModelRc,VecModel,SharedString};
use std::rc::Rc;
slint::slint! { import { MainWindow } from "../ui/main.slint"; }
struct Offscreen(Rc<MinimalSoftwareWindow>);
impl Platform for Offscreen {fn create_window_adapter(&self)->Result<Rc<dyn WindowAdapter>,slint::PlatformError>{Ok(self.0.clone())}}
fn capture(w:&MinimalSoftwareWindow,path:&str,width:usize,height:usize){
 use std::io::Write;
 w.request_redraw(); let mut pixels=vec![Rgb8Pixel::default();width*height];
 w.draw_if_needed(|r|{r.render(&mut pixels,width);});
 let stride=(width*3+3)&!3;let mut f=std::fs::File::create(path).unwrap();let mut h=vec![0u8;54];
 h[0..2].copy_from_slice(b"BM");h[2..6].copy_from_slice(&((54+stride*height)as u32).to_le_bytes());h[10..14].copy_from_slice(&54u32.to_le_bytes());h[14..18].copy_from_slice(&40u32.to_le_bytes());h[18..22].copy_from_slice(&(width as i32).to_le_bytes());h[22..26].copy_from_slice(&(height as i32).to_le_bytes());h[26..28].copy_from_slice(&1u16.to_le_bytes());h[28..30].copy_from_slice(&24u16.to_le_bytes());f.write_all(&h).unwrap();
 for y in (0..height).rev(){let mut row=vec![0u8;stride];for x in 0..width{let p=pixels[y*width+x];row[x*3..x*3+3].copy_from_slice(&[p.b,p.g,p.r]);}f.write_all(&row).unwrap();}
}
fn labels(page:&andromeda_core::catalog::CatalogPage<andromeda_core::catalog::McVersion>)->Vec<SharedString>{
 page.items.iter().map(|v|{let k=match v.kind{McKind::Release=>"release",McKind::Snapshot=>"snapshot",McKind::OldBeta=>"old beta",McKind::OldAlpha=>"old alpha"};SharedString::from(format!("{} · {}",v.id,k))}).collect()
}
fn main()->Result<(),slint::PlatformError>{
 let dl=andromeda_core::vanilla::Downloader::new().unwrap();
 std::fs::create_dir_all("profiles-proof/catalog-cache").unwrap();
 let cache=std::path::Path::new("profiles-proof/catalog-cache");
 let f_rel=McFilter{release:true,snapshot:false,old_beta:false,old_alpha:false};
 let f_snap=McFilter{release:false,snapshot:true,old_beta:false,old_alpha:false};
 let f_old=McFilter{release:false,snapshot:false,old_beta:true,old_alpha:true};
 // Functional assertions on the real catalog behind each filter chip.
 let rel=mc_versions(&dl,cache,f_rel).unwrap();
 let snap=mc_versions(&dl,cache,f_snap).unwrap();
 let old=mc_versions(&dl,cache,f_old).unwrap();
 assert!(rel.items.iter().all(|v|v.kind==McKind::Release) && !rel.items.is_empty());
 assert!(snap.items.iter().all(|v|v.kind==McKind::Snapshot) && !snap.items.is_empty());
 assert!(old.items.iter().all(|v|matches!(v.kind,McKind::OldBeta|McKind::OldAlpha)) && !old.items.is_empty());
 assert_ne!(rel.items[0].id,snap.items[0].id);
 println!("filters live: {} stable (top {}), {} snapshots (top {}), {} old (top {})",rel.items.len(),rel.items[0].id,snap.items.len(),snap.items[0].id,old.items.len(),old.items[0].id);
 let w=MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);slint::platform::set_platform(Box::new(Offscreen(w.clone()))).unwrap();
 let ui=MainWindow::new()?;ui.window().set_size(slint::PhysicalSize::new(1600,900));
 ui.set_reduce_animations(true);ui.set_active_tab(3);ui.set_settings_section(0);
 // Callback path: the refresh handler fed by the same live calls.
 let ui_weak=ui.as_weak();
 ui.on_version_refresh(move |f|{
  let Some(ui)=ui_weak.upgrade() else { return };
  let dl=andromeda_core::vanilla::Downloader::new().unwrap();
  let filter=match f{0=>f_rel,1=>f_snap,2=>f_old,_=>McFilter::all()};
  let page=mc_versions(&dl,std::path::Path::new("profiles-proof/catalog-cache"),filter).unwrap();
  ui.set_version_choices(ModelRc::new(VecModel::from(labels(&page))));
 });
 ui.invoke_version_refresh(0);
 let stable_top=ui.get_version_choices().row_data(0).unwrap_or_default().to_string();
 ui.invoke_version_refresh(1);
 let snap_top=ui.get_version_choices().row_data(0).unwrap_or_default().to_string();
 assert_ne!(stable_top,snap_top,"filter callback must change the listed versions");
 println!("callback proof: stable chip lists '{}' first, snapshots chip lists '{}'",stable_top,snap_top);
 // State 1: stable filter, newest release, Fabric picked with its live builds.
 let mc=rel.items[0].id.clone();
 let avail=available_loaders(&dl,cache,&mc).unwrap();
 let flags:Vec<bool>=[true].into_iter().chain(LoaderKind::all().iter().map(|k|avail.contains(k))).collect();
 println!("loaders for {}: {:?}",mc,avail);
 let builds=loader_versions(&dl,cache,LoaderKind::Fabric,&mc).unwrap();
 let blabels:Vec<SharedString>=builds.items.iter().map(|v|SharedString::from(if v.stable==Some(true){format!("{} · stable",v.version)}else{v.version.clone()})).collect();
 println!("fabric builds for {}: {} (top {})",mc,builds.items.len(),builds.items[0].version);
 ui.invoke_version_refresh(0);
 ui.set_version_filter(0);ui.set_version_index(0);
 ui.set_loader_available(ModelRc::new(VecModel::from(flags.clone())));
 ui.set_loader_index(1);
 ui.set_loader_version_choices(ModelRc::new(VecModel::from(blabels)));ui.set_loader_version_index(0);
 ui.show()?;
 capture(&w,"profiles-proof/picker-stable-fabric.bmp",1600,900);
 // State 2: snapshots chip with real snapshot data, Vanilla.
 ui.set_loader_index(0);ui.set_version_filter(1);ui.set_loader_version_choices(ModelRc::new(VecModel::from(Vec::<SharedString>::new())));
 let mc_snap=snap.items[0].id.clone();
 let avail_snap=available_loaders(&dl,cache,&mc_snap).unwrap();
 let flags_snap:Vec<bool>=[true].into_iter().chain(LoaderKind::all().iter().map(|k|avail_snap.contains(k))).collect();
 println!("loaders for {}: {:?}",mc_snap,avail_snap);
 ui.set_loader_available(ModelRc::new(VecModel::from(flags_snap)));
 capture(&w,"profiles-proof/picker-snapshots.bmp",1600,900);
 // State 3: an old beta without any loader builds: loader row disabled.
 ui.set_version_choices(ModelRc::new(VecModel::from(labels(&old))));
 ui.set_version_filter(2);
 let beta=old.items.iter().position(|v|v.id=="b1.7.3").unwrap_or(0);
 let mc_old=old.items[beta].id.clone();
 let avail_old=available_loaders(&dl,cache,&mc_old).unwrap();
 let flags_old:Vec<bool>=[true].into_iter().chain(LoaderKind::all().iter().map(|k|avail_old.contains(k))).collect();
 println!("loaders for {}: {:?} (loader row must be disabled)",mc_old,avail_old);
 ui.set_version_index(beta as i32);ui.set_loader_available(ModelRc::new(VecModel::from(flags_old)));ui.set_loader_index(0);
 capture(&w,"profiles-proof/picker-old-no-loaders.bmp",1600,900);
 println!("Profiles picker proof: 3 settled states with live data, 1600x900. No real window opened.");Ok(())
}
