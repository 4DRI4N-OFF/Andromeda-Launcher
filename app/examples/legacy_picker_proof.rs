//! Legacy loader picker proof with LIVE catalog data and REAL input events.
//! Scenario: select release 1.8.9 (a LegacyFabric-covered Minecraft) through
//! the real callbacks, confirm the availability flags enable the sixth loader
//! button, click it at its rendered position, and confirm the LegacyFabric
//! build list loads with the newest published build on top. Disabled buttons
//! from loaders that do not publish for 1.8.9 must keep ignoring clicks.
//! Settled rendering, no real window opened. Capture: picker-legacyfabric-1600.bmp.
use andromeda_core::catalog::{available_loaders, loader_versions, mc_versions, LoaderKind, McFilter, McKind};
use slint::{platform::{software_renderer::{MinimalSoftwareWindow,RepaintBufferType},Platform,WindowAdapter,WindowEvent,PointerEventButton},ComponentHandle,LogicalPosition,Model,Rgb8Pixel,ModelRc,VecModel,SharedString};
use std::rc::Rc;
slint::slint! { import { MainWindow } from "../ui/main.slint"; }
struct Offscreen(Rc<MinimalSoftwareWindow>);
impl Platform for Offscreen {fn create_window_adapter(&self)->Result<Rc<dyn WindowAdapter>,slint::PlatformError>{Ok(self.0.clone())}}
fn render(w:&MinimalSoftwareWindow,width:usize,height:usize)->Vec<Rgb8Pixel>{w.request_redraw();let mut p=vec![Rgb8Pixel::default();width*height];w.draw_if_needed(|r|{r.render(&mut p,width);});p}
fn bmp(pixels:&[Rgb8Pixel],path:&str,width:usize,height:usize){
 use std::io::Write;
 let stride=(width*3+3)&!3;let mut f=std::fs::File::create(path).unwrap();let mut h=vec![0u8;54];
 h[0..2].copy_from_slice(b"BM");h[2..6].copy_from_slice(&((54+stride*height)as u32).to_le_bytes());h[10..14].copy_from_slice(&54u32.to_le_bytes());h[14..18].copy_from_slice(&40u32.to_le_bytes());h[18..22].copy_from_slice(&(width as i32).to_le_bytes());h[22..26].copy_from_slice(&(height as i32).to_le_bytes());h[26..28].copy_from_slice(&1u16.to_le_bytes());h[28..30].copy_from_slice(&24u16.to_le_bytes());f.write_all(&h).unwrap();
 for y in (0..height).rev(){let mut row=vec![0u8;stride];for x in 0..width{let p=pixels[y*width+x];row[x*3..x*3+3].copy_from_slice(&[p.b,p.g,p.r]);}f.write_all(&row).unwrap();}
}
fn pump(){slint::platform::update_timers_and_animations();}
fn wheel(ui:&MainWindow,x:f32,y:f32,dy:f32){let w=ui.window();w.dispatch_event(WindowEvent::PointerScrolled{position:LogicalPosition::new(x,y),delta_x:0.,delta_y:dy});pump();}
fn click(ui:&MainWindow,x:f32,y:f32){let w=ui.window();let p=LogicalPosition::new(x,y);
 w.dispatch_event(WindowEvent::PointerMoved{position:p});
 w.dispatch_event(WindowEvent::PointerPressed{position:p,button:PointerEventButton::Left});
 w.dispatch_event(WindowEvent::PointerReleased{position:p,button:PointerEventButton::Left});pump();}
fn main()->Result<(),slint::PlatformError>{
 let dl=andromeda_core::vanilla::Downloader::new().unwrap();
 std::fs::create_dir_all("profiles-proof/catalog-cache").unwrap();
 let cache=std::path::Path::new("profiles-proof/catalog-cache");
 let f_rel=McFilter{release:true,snapshot:false,old_beta:false,old_alpha:false};
 let rel=mc_versions(&dl,cache,f_rel).unwrap();
 let idx_189=rel.items.iter().position(|v|v.id=="1.8.9").expect("1.8.9 must be a listed release");
 // Live ground truth for the assertions below.
 let avail=available_loaders(&dl,cache,"1.8.9").unwrap();
 assert!(avail.contains(&LoaderKind::LegacyFabric),"1.8.9 must offer LegacyFabric");
 println!("live: 1.8.9 available loaders {:?}",avail);
 let lf_page=loader_versions(&dl,cache,LoaderKind::LegacyFabric,"1.8.9").unwrap();
 let lf_top=lf_page.items.first().map(|v|v.version.clone()).unwrap_or_default();
 assert!(!lf_top.is_empty(),"LegacyFabric must publish builds for 1.8.9");
 println!("live: LegacyFabric builds for 1.8.9 -> {} listed (top {})",lf_page.items.len(),lf_top);
 let w=MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);slint::platform::set_platform(Box::new(Offscreen(w.clone()))).unwrap();
 let ui=MainWindow::new()?;ui.window().set_size(slint::PhysicalSize::new(1600,900));
 ui.set_reduce_animations(true);ui.set_active_tab(3);ui.set_settings_section(0);
 {let wk=ui.as_weak();ui.on_version_refresh(move |f|{let Some(u)=wk.upgrade()else{return};
  let dl=andromeda_core::vanilla::Downloader::new().unwrap();
  let filter=match f{0=>McFilter{release:true,snapshot:false,old_beta:false,old_alpha:false},1=>McFilter{release:false,snapshot:true,old_beta:false,old_alpha:false},2=>McFilter{release:false,snapshot:false,old_beta:true,old_alpha:true},_=>McFilter::all()};
  let page=mc_versions(&dl,std::path::Path::new("profiles-proof/catalog-cache"),filter).unwrap();
  let labels:Vec<SharedString>=page.items.iter().map(|v|{let k=match v.kind{McKind::Release=>"release",McKind::Snapshot=>"snapshot",McKind::OldBeta=>"old beta",McKind::OldAlpha=>"old alpha"};SharedString::from(format!("{} · {}",v.id,k))}).collect();
  u.set_version_choices(ModelRc::new(VecModel::from(labels)));});}
 // version-picked: availability refresh, mirroring the app wiring.
 {let wk=ui.as_weak();ui.on_version_picked(move |i|{let Some(u)=wk.upgrade()else{return};
  let Some(row)=u.get_version_choices().row_data(i as usize)else{return};
  let mc=row.split(' ').next().unwrap_or_default().to_string();
  let dl=andromeda_core::vanilla::Downloader::new().unwrap();
  let avail=available_loaders(&dl,std::path::Path::new("profiles-proof/catalog-cache"),&mc).unwrap();
  let flags:Vec<bool>=[true].into_iter().chain(LoaderKind::all().iter().map(|k|avail.contains(k))).collect();
  if u.get_loader_index()>0&&!flags.get(u.get_loader_index() as usize).copied().unwrap_or(false){u.set_loader_index(0);}
  u.set_loader_available(ModelRc::new(VecModel::from(flags)));});}
 // loader-picked: build list for enabled kinds, cleared for Vanilla.
 {let wk=ui.as_weak();ui.on_loader_picked(move |i|{let Some(u)=wk.upgrade()else{return};
  if i==0{u.set_loader_version_choices(ModelRc::new(VecModel::from(Vec::<SharedString>::new())));return;}
  let Some(row)=u.get_version_choices().row_data(u.get_version_index() as usize)else{return};
  let mc=row.split(' ').next().unwrap_or_default().to_string();
  let kind=[LoaderKind::Fabric,LoaderKind::Quilt,LoaderKind::Forge,LoaderKind::NeoForge,LoaderKind::LegacyFabric][i as usize-1];
  let dl=andromeda_core::vanilla::Downloader::new().unwrap();
  let page=loader_versions(&dl,std::path::Path::new("profiles-proof/catalog-cache"),kind,&mc).unwrap();
  let labels:Vec<SharedString>=page.items.iter().map(|v|SharedString::from(if v.stable==Some(true){format!("{} · stable",v.version)}else{v.version.clone()})).collect();
  println!("loader-picked event: {:?} builds for {} -> {} listed (top {})",kind,mc,page.items.len(),page.items.first().map(|v|v.version.as_str()).unwrap_or("-"));
  u.set_loader_version_index(0);u.set_loader_version_choices(ModelRc::new(VecModel::from(labels)));});}
 // Stable filter through the real callback, then pick 1.8.9 by its live index.
 ui.set_version_filter(0);ui.invoke_version_refresh(0);
 assert_eq!(ui.get_version_choices().row_count(),rel.items.len(),"release list must match the live catalog");
 ui.set_version_index(idx_189 as i32);ui.invoke_version_picked(idx_189 as i32);
 let flags=ui.get_loader_available();
 assert_eq!(flags.row_count(),6,"six loader buttons must be flagged");
 assert!(flags.row_data(5).unwrap_or(false),"LegacyFabric button must be enabled for 1.8.9");
 assert!(!flags.row_data(1).unwrap_or(true),"Fabric must stay disabled for 1.8.9 (starts at 1.14)");
 assert!(!flags.row_data(4).unwrap_or(true),"NeoForge must stay disabled for 1.8.9");
 println!("event proof: 1.8.9 picked -> loader flags [vanilla,fabric,quilt,forge,neoforge,legacyfabric] = {:?}",
  (0..6).map(|i|flags.row_data(i).unwrap_or(false)).collect::<Vec<_>>());
 ui.show()?;pump();
 // Click the LegacyFabric button at its rendered position. It is the
 // rightmost of six stretched buttons; scan from the right edge inward and
 // stop when the real click selects it. Disabled neighbors ignore clicks.
 let mut picked=false;
 for x in [1570.,1540.,1510.,1480.,1450.,1420.,1390.,1360.,1330.]{
  click(&ui,x,719.);
  if ui.get_loader_index()==5{picked=true;println!("event proof: click at x={x:.0}, y=719 -> loader = LegacyFabric");break;}
 }
 assert!(picked,"LegacyFabric button must accept a real click within its row");
 // The click routed through loader-picked: the build list must hold the live
 // LegacyFabric builds with the newest on top.
 let builds=ui.get_loader_version_choices();
 assert!(builds.row_count()>0,"LegacyFabric build list must not be empty");
 let top=builds.row_data(0).unwrap_or_default().to_string();
 assert!(top.starts_with(lf_top.as_str()),"top build must be the live newest ({lf_top}), got {top}");
 println!("event proof: build list top '{top}' matches live LegacyFabric metadata");
 // A disabled neighbor (Fabric, no 1.14- coverage) must ignore its click.
 for x in [700.,730.,670.,760.]{
  click(&ui,x,719.);
  if ui.get_loader_index()==1{panic!("disabled Fabric accepted a click at x={x}");}
 }
 assert_eq!(ui.get_loader_index(),5,"loader stays LegacyFabric after clicking disabled neighbors");
 println!("event proof: clicks on disabled neighbors ignored; loader stays LegacyFabric");
 let p=render(&w,1600,900);bmp(&p,"profiles-proof/picker-legacyfabric-1600.bmp",1600,900);
 println!("capture: profiles-proof/picker-legacyfabric-1600.bmp");
 // Scroll the inner version list until the selected 1.8.9 row is in view,
 // then prove its label paints (accent outline = selected row, as in the
 // build-list traversal of the picker proof v3).
 let mut shown=false;
 for _ in 0..40{
  wheel(&ui,1000.,580.,-240.);
  let p=render(&w,1600,900);
  let mut bands:Vec<(usize,usize)>=vec![];
  for y in 500..655{let mut run=0;for x in 0..1600{let q=p[y*1600+x];if q.r>200&&(150..215).contains(&q.g)&&q.b<170{run+=1;}}
   if run>400{if let Some(l)=bands.last_mut(){if y-l.1<=1{l.1=y;continue;}}bands.push((y,y));}}
  let thin:Vec<(usize,usize)>=bands.into_iter().filter(|b|b.1-b.0<4).collect();
  if thin.len()>=2{
   let by=thin[thin.len()-1].0;
   let label:usize=(by.saturating_sub(40)..by.saturating_sub(6)).map(|y|(520..1500).filter(|&x|{let q=p[y*1600+x];q.r>180&&q.g>180&&q.b>180}).count()).sum();
   println!("version list: selected 1.8.9 row bottom edge at y={by}; label pixels={label}");
   assert!(label>40,"selected 1.8.9 row must paint its label");
   bmp(&p,"profiles-proof/picker-legacyfabric-1600.bmp",1600,900);
   shown=true;break;}}
 assert!(shown,"selected 1.8.9 row must be reachable by scrolling the version list");
 println!("capture: profiles-proof/picker-legacyfabric-1600.bmp (1.8.9 row in view)");
 println!("Legacy picker proof complete: live data, real events, 1 capture.");Ok(())
}
