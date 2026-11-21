#![allow(deprecated)] // slint inline-import emits a deprecated fallback const in this proof target; no behavior change
//! LiteLoader loader picker proof with LIVE catalog data and REAL input events.
//! Scenario: select release 1.12.2 through the real callbacks, confirm the
//! availability flags match the live catalog (LiteLoader must be among them),
//! click LiteLoader (the ninth, rightmost button) at its rendered position,
//! and confirm its single published build loads. Then prove exclusive
//! selection: clicking another enabled loader replaces the selection and its
//! build list, clicking LiteLoader again replaces it back; disabled loaders
//! ignore clicks. Repeated at 1024x768 and 860x620 (the nine-button row under
//! pressure). Settled rendering, no real window opened.
//! Captures: picker-liteloader-1600.bmp, picker-liteloader-1024.bmp,
//! picker-liteloader-860.bmp.
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
 let idx_1122=rel.items.iter().position(|v|v.id=="1.12.2").expect("1.12.2 must be a listed release");
 // Live ground truth for the assertions below.
 let avail=available_loaders(&dl,cache,"1.12.2").unwrap();
 assert!(avail.contains(&LoaderKind::LiteLoader),"1.12.2 must offer LiteLoader");
 println!("live: 1.12.2 available loaders {:?}",avail);
 let lt_page=loader_versions(&dl,cache,LoaderKind::LiteLoader,"1.12.2").unwrap();
 let lt_top=lt_page.items.first().map(|v|v.version.clone()).unwrap_or_default();
 assert_eq!(lt_page.items.len(),1,"LiteLoader publishes one current build per game");
 assert!(!lt_top.is_empty(),"LiteLoader must publish a build for 1.12.2");
 println!("live: LiteLoader build for 1.12.2 -> {lt_top}");
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
  let kind=[LoaderKind::Fabric,LoaderKind::Quilt,LoaderKind::Forge,LoaderKind::NeoForge,LoaderKind::LegacyFabric,LoaderKind::Babric,LoaderKind::Ornithe,LoaderKind::LiteLoader][i as usize-1];
  let dl=andromeda_core::vanilla::Downloader::new().unwrap();
  let page=loader_versions(&dl,std::path::Path::new("profiles-proof/catalog-cache"),kind,&mc).unwrap();
  let labels:Vec<SharedString>=page.items.iter().map(|v|SharedString::from(if v.stable==Some(true){format!("{} · stable",v.version)}else{v.version.clone()})).collect();
  println!("loader-picked event: {:?} builds for {} -> {} listed (top {})",kind,mc,page.items.len(),page.items.first().map(|v|v.version.as_str()).unwrap_or("-"));
  u.set_loader_version_index(0);u.set_loader_version_choices(ModelRc::new(VecModel::from(labels)));});}
 // Release filter through the real callback, then pick 1.12.2 by its live index.
 ui.set_version_filter(0);ui.invoke_version_refresh(0);
 assert_eq!(ui.get_version_choices().row_count(),rel.items.len(),"release list must match the live catalog");
 ui.set_version_index(idx_1122 as i32);ui.invoke_version_picked(idx_1122 as i32);
 let flags=ui.get_loader_available();
 assert_eq!(flags.row_count(),9,"nine loader buttons must be flagged");
 // Every flag must equal the live availability for its kind.
 for (i,k) in LoaderKind::all().iter().enumerate(){
  assert_eq!(flags.row_data(i+1).unwrap_or(false),avail.contains(k),"flag for {:?} must match live availability",k);
 }
 assert!(flags.row_data(8).unwrap_or(false),"LiteLoader button must be enabled for 1.12.2");
 println!("event proof: 1.12.2 picked -> loader flags [vanilla,fabric,quilt,forge,neoforge,legacyfabric,babric,ornithe,liteloader] = {:?}",
  (0..9).map(|i|flags.row_data(i).unwrap_or(false)).collect::<Vec<_>>());
 ui.show()?;pump();
 // Click the LiteLoader button at its rendered position. It is the rightmost
 // of nine stretched buttons; scan from the right edge inward and stop when
 // the real click selects it.
 let mut lt_x=0.0f32;let mut lt_y=0.0f32;
 'found: for y in [719.,700.,740.,680.,760.,640.]{
  for x in [1570.,1540.,1510.,1480.,1450.,1420.,1390.,1360.,1330.,1300.,1270.]{
   click(&ui,x,y);
   if ui.get_loader_index()==8{lt_x=x;lt_y=y;println!("event proof: click at x={x:.0}, y={y:.0} -> loader = LiteLoader");break 'found;}
   if ui.get_loader_index()!=0{panic!("an unexpected loader accepted a click at x={x} y={y}");}
  }
 }
 assert!(lt_x>0.,"LiteLoader button must accept a real click within its row");
 let builds=ui.get_loader_version_choices();
 assert_eq!(builds.row_count(),1,"LiteLoader build list must match the live list");
 let top=builds.row_data(0).unwrap_or_default().to_string();
 assert!(top.starts_with(lt_top.as_str()),"top build must be the live one ({lt_top}), got {top}");
 println!("event proof: build list '{top}' matches live LiteLoader metadata");
 // Exclusive selection: clicking another enabled loader replaces LiteLoader.
 // Forge (index 3) publishes for 1.12.2; scan left until it accepts.
 let mut other_x=0.0f32;
 let mut x=lt_x-40.;
 while x>600.{
  click(&ui,x,lt_y);
  let idx=ui.get_loader_index();
  if idx!=8&&idx!=0{other_x=x;println!("event proof: click at x={x:.0}, y={lt_y:.0} -> loader index {idx} (replaced LiteLoader)");break;}
  if idx!=8{panic!("an unexpected loader accepted a click at x={x}");}
  x-=30.;
 }
 assert!(other_x>0.,"another enabled loader must accept a real click within the row");
 let replaced=ui.get_loader_version_choices();
 assert!(replaced.row_count()>0,"the replacing loader must list its live builds");
 assert!(replaced.row_data(0).unwrap_or_default()!=top,"build list must be replaced by the other loader's list");
 println!("event proof: selection is exclusive - another loader replaced LiteLoader, list swapped");
 // And back: LiteLoader replaces it again.
 click(&ui,lt_x,lt_y);
 assert_eq!(ui.get_loader_index(),8,"clicking LiteLoader again must replace the other loader");
 let builds=ui.get_loader_version_choices();
 assert_eq!(builds.row_count(),1,"build list must switch back to LiteLoader");
 assert!(builds.row_data(0).unwrap_or_default().to_string().starts_with(lt_top.as_str()),"top build must switch back to live LiteLoader build");
 println!("event proof: LiteLoader selected again; list swapped back ({lt_top})");
 // Disabled neighbors must ignore their clicks (Fabric/Quilt/NeoForge/Babric
 // publish nothing for 1.12.2).
 for (i,k) in LoaderKind::all().iter().enumerate(){
  if !avail.contains(k){
   let mut x=lt_x-20.;
   while x>600.{
    click(&ui,x,lt_y);
    let idx=ui.get_loader_index();
    if idx==i as i32+1{panic!("disabled loader {k:?} accepted a click at x={x}");}
    if idx!=8{click(&ui,lt_x,lt_y);assert_eq!(ui.get_loader_index(),8);}
    x-=60.;
   }
  }
 }
 assert_eq!(ui.get_loader_index(),8,"loader stays LiteLoader after clicking disabled neighbors");
 println!("event proof: clicks on disabled loaders ignored; loader stays LiteLoader");
 // Bring the selected 1.12.2 row into the version-list viewport so the
 // capture identifies the picked version, not only loader/build. Scroll
 // offset persists across the window resizes below.
 let target_scroll=idx_1122 as f32*48.-96.; // SectionRow pitch is 48px; put 1.12.2 second in the viewport
 let vp=LogicalPosition::new(900.,520.);
 ui.window().dispatch_event(WindowEvent::PointerScrolled{position:vp,delta_x:0.,delta_y:-target_scroll});
 pump();
 let p=render(&w,1600,900);bmp(&p,"profiles-proof/picker-liteloader-1600.bmp",1600,900);
 println!("capture: profiles-proof/picker-liteloader-1600.bmp");
 // Smaller window: the nine-button row must still work at 1024x768.
 ui.window().set_size(slint::PhysicalSize::new(1024,768));pump();
 ui.set_loader_index(0);ui.invoke_loader_picked(0);
 let mut small=false;
 'outer: for y in [719.,700.,740.,680.,760.,640.,790.]{
  let mut x=1000.;
  while x>700.{
   click(&ui,x,y);
   if ui.get_loader_index()==8{println!("event proof: 1024px click at x={x:.0}, y={y:.0} -> loader = LiteLoader");small=true;break 'outer;}
   if ui.get_loader_index()!=0{panic!("an unexpected loader accepted a 1024px click at x={x} y={y}");}
   x-=20.;
  }
 }
 assert!(small,"LiteLoader button must accept a real click at 1024x768");
 let builds=ui.get_loader_version_choices();
 assert_eq!(builds.row_count(),1,"LiteLoader build list must load at 1024x768");
 assert!(builds.row_data(0).unwrap_or_default().to_string().starts_with(lt_top.as_str()),"top build must be the live one at 1024x768");
 let p=render(&w,1024,768);bmp(&p,"profiles-proof/picker-liteloader-1024.bmp",1024,768);
 println!("capture: profiles-proof/picker-liteloader-1024.bmp (LiteLoader selected at 1024x768)");
 // 860x620: the previous minimum target, now with nine buttons. The loader
 // row sits below the fold here, so scroll the settings panel first (real
 // scroll events over the ScrollView), then click inside the window.
 ui.window().set_size(slint::PhysicalSize::new(860,620));pump();
 ui.set_loader_index(0);ui.invoke_loader_picked(0);
 // At 860 the window bottom clips the list's third row: nudge the list one
 // row further so the picked 1.12.2 sits fully inside the viewport.
 let vp2=LogicalPosition::new(500.,520.);
 ui.window().dispatch_event(WindowEvent::PointerScrolled{position:vp2,delta_x:0.,delta_y:-48.});
 pump();
 // At 860 the version list and the loader row do not fit together: capture
 // the picked 1.12.2 row first (panel at top), then scroll for the row.
 let pv=render(&w,860,620);bmp(&pv,"profiles-proof/picker-liteloader-860-versions.bmp",860,620);
 println!("capture: profiles-proof/picker-liteloader-860-versions.bmp (1.12.2 selected in the list at 860x620)");
 let sp=LogicalPosition::new(600.,400.);
 for _ in 0..10{
  ui.window().dispatch_event(WindowEvent::PointerScrolled{position:sp,delta_x:0.,delta_y:-60.});
 }
 pump();
 let mut tiny=false;
 'outer860: for y in [272.,260.,285.,250.,300.,240.,310.,230.,320.,220.,330.,210.,340.,200.,350.,190.]{
  let mut x=790.;
  while x>380.{
   click(&ui,x,y);
   if ui.get_loader_index()==8{println!("event proof: 860px click after scroll at x={x:.0}, y={y:.0} -> loader = LiteLoader");tiny=true;break 'outer860;}
   if ui.get_loader_index()!=0{panic!("an unexpected loader accepted a 860px click at x={x} y={y}");}
   x-=20.;
  }
 }
 assert!(tiny,"LiteLoader button must be reachable by scroll + real click at 860x620");
 let builds=ui.get_loader_version_choices();
 assert_eq!(builds.row_count(),1,"LiteLoader build list must load at 860x620");
 assert!(builds.row_data(0).unwrap_or_default().to_string().starts_with(lt_top.as_str()),"top build must be the live one at 860x620");
 let p=render(&w,860,620);bmp(&p,"profiles-proof/picker-liteloader-860.bmp",860,620);
 println!("capture: profiles-proof/picker-liteloader-860.bmp (860x620, nine-button row)");
 println!("LiteLoader picker proof complete: live data, real events, exclusive selection, 3 captures.");Ok(())
}

