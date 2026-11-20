//! Ornithe loader picker proof with LIVE catalog data and REAL input events.
//! Scenario: select beta 1.7.3 through the Old filter and the real callbacks,
//! confirm the availability flags enable exactly the Babric and Ornithe
//! loader buttons, click Ornithe at its rendered position, and confirm the
//! Ornithe build list loads with the newest published build on top. Then
//! prove exclusive selection: clicking Babric replaces the selection and its
//! build list, clicking Ornithe again replaces it back; disabled loaders
//! ignore clicks. Repeated at a smaller window size. Settled rendering, no
//! real window opened.
//! Captures: picker-ornithe-1600.bmp, picker-ornithe-1024.bmp.
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
 let f_old=McFilter{release:false,snapshot:false,old_beta:true,old_alpha:true};
 let old=mc_versions(&dl,cache,f_old).unwrap();
 let idx_b173=old.items.iter().position(|v|v.id=="b1.7.3").expect("b1.7.3 must be a listed old beta");
 // Live ground truth for the assertions below.
 let avail=available_loaders(&dl,cache,"b1.7.3").unwrap();
 assert_eq!(avail,vec![LoaderKind::Babric,LoaderKind::Ornithe],"b1.7.3 must offer Babric and Ornithe");
 println!("live: b1.7.3 available loaders {:?}",avail);
 let or_page=loader_versions(&dl,cache,LoaderKind::Ornithe,"b1.7.3").unwrap();
 let or_top=or_page.items.first().map(|v|v.version.clone()).unwrap_or_default();
 assert!(!or_top.is_empty(),"Ornithe must publish builds for b1.7.3");
 println!("live: Ornithe builds for b1.7.3 -> {} listed (top {})",or_page.items.len(),or_top);
 let bb_page=loader_versions(&dl,cache,LoaderKind::Babric,"b1.7.3").unwrap();
 let bb_top=bb_page.items.first().map(|v|v.version.clone()).unwrap_or_default();
 assert!(!bb_top.is_empty(),"Babric must publish builds for b1.7.3");
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
  let kind=[LoaderKind::Fabric,LoaderKind::Quilt,LoaderKind::Forge,LoaderKind::NeoForge,LoaderKind::LegacyFabric,LoaderKind::Babric,LoaderKind::Ornithe][i as usize-1];
  let dl=andromeda_core::vanilla::Downloader::new().unwrap();
  let page=loader_versions(&dl,std::path::Path::new("profiles-proof/catalog-cache"),kind,&mc).unwrap();
  let labels:Vec<SharedString>=page.items.iter().map(|v|SharedString::from(if v.stable==Some(true){format!("{} · stable",v.version)}else{v.version.clone()})).collect();
  println!("loader-picked event: {:?} builds for {} -> {} listed (top {})",kind,mc,page.items.len(),page.items.first().map(|v|v.version.as_str()).unwrap_or("-"));
  u.set_loader_version_index(0);u.set_loader_version_choices(ModelRc::new(VecModel::from(labels)));});}
 // Old filter through the real callback, then pick b1.7.3 by its live index.
 ui.set_version_filter(2);ui.invoke_version_refresh(2);
 assert_eq!(ui.get_version_choices().row_count(),old.items.len(),"old list must match the live catalog");
 ui.set_version_index(idx_b173 as i32);ui.invoke_version_picked(idx_b173 as i32);
 let flags=ui.get_loader_available();
 assert_eq!(flags.row_count(),8,"eight loader buttons must be flagged");
 assert!(flags.row_data(6).unwrap_or(false),"Babric button must be enabled for b1.7.3");
 assert!(flags.row_data(7).unwrap_or(false),"Ornithe button must be enabled for b1.7.3");
 for i in 1..6{assert!(!flags.row_data(i).unwrap_or(true),"loader {i} must stay disabled for b1.7.3");}
 println!("event proof: b1.7.3 picked -> loader flags [vanilla,fabric,quilt,forge,neoforge,legacyfabric,babric,ornithe] = {:?}",
  (0..8).map(|i|flags.row_data(i).unwrap_or(false)).collect::<Vec<_>>());
 ui.show()?;pump();
 // Click the Ornithe button at its rendered position. It is the rightmost of
 // eight stretched buttons; scan from the right edge inward and stop when
 // the real click selects it.
 let mut or_x=0.0f32;
 for x in [1570.,1540.,1510.,1480.,1450.,1420.,1390.,1360.,1330.,1300.,1270.]{
  click(&ui,x,719.);
  if ui.get_loader_index()==7{or_x=x;println!("event proof: click at x={x:.0}, y=719 -> loader = Ornithe");break;}
 }
 assert!(or_x>0.,"Ornithe button must accept a real click within its row");
 let builds=ui.get_loader_version_choices();
 assert_eq!(builds.row_count(),or_page.items.len(),"Ornithe build list must match the live list");
 let top=builds.row_data(0).unwrap_or_default().to_string();
 assert!(top.starts_with(or_top.as_str()),"top build must be the live newest ({or_top}), got {top}");
 println!("event proof: build list top '{top}' matches live Ornithe metadata");
 // Exclusive selection: clicking Babric replaces Ornithe and its build list.
 let mut bb_x=0.0f32;
 let mut x=or_x-40.;
 while x>900.{
  click(&ui,x,719.);
  if ui.get_loader_index()==6{bb_x=x;println!("event proof: click at x={x:.0}, y=719 -> loader = Babric (replaced Ornithe)");break;}
  if ui.get_loader_index()!=7{panic!("an unexpected loader accepted a click at x={x}");}
  x-=30.;
 }
 assert!(bb_x>0.,"Babric button must accept a real click within its row");
 let builds=ui.get_loader_version_choices();
 assert_eq!(builds.row_count(),bb_page.items.len(),"build list must be replaced by the live Babric list");
 assert!(builds.row_data(0).unwrap_or_default().to_string().starts_with(bb_top.as_str()),"top build must switch to live Babric newest");
 println!("event proof: selection is exclusive - Babric replaced Ornithe, list swapped (top {bb_top})");
 // And back: Ornithe replaces Babric again.
 click(&ui,or_x,719.);
 assert_eq!(ui.get_loader_index(),7,"clicking Ornithe again must replace Babric");
 let builds=ui.get_loader_version_choices();
 assert_eq!(builds.row_count(),or_page.items.len(),"build list must switch back to Ornithe");
 assert!(builds.row_data(0).unwrap_or_default().to_string().starts_with(or_top.as_str()),"top build must switch back to live Ornithe newest");
 println!("event proof: Ornithe selected again; list swapped back (top {or_top})");
 // Disabled neighbors must ignore their clicks.
 for x in [700.,730.,670.,760.,900.,1100.,1180.]{
  click(&ui,x,719.);
  if ui.get_loader_index()!=7{panic!("a disabled loader accepted a click at x={x}");}
 }
 assert_eq!(ui.get_loader_index(),7,"loader stays Ornithe after clicking disabled neighbors");
 println!("event proof: clicks on disabled neighbors ignored; loader stays Ornithe");
 let p=render(&w,1600,900);bmp(&p,"profiles-proof/picker-ornithe-1600.bmp",1600,900);
 println!("capture: profiles-proof/picker-ornithe-1600.bmp");
 // Smaller window: the eight-button row must still work at 1024x768.
 ui.window().set_size(slint::PhysicalSize::new(1024,768));pump();
 ui.set_loader_index(0);ui.invoke_loader_picked(0);
 let mut small=false;
 'outer: for y in [719.,700.,740.,680.,760.,640.,790.]{
  let mut x=1000.;
  while x>820.{
   click(&ui,x,y);
   if ui.get_loader_index()==7{println!("event proof: 1024px click at x={x:.0}, y={y:.0} -> loader = Ornithe");small=true;break 'outer;}
   if ui.get_loader_index()!=0{panic!("an unexpected loader accepted a 1024px click at x={x} y={y}");}
   x-=20.;
  }
 }
 assert!(small,"Ornithe button must accept a real click at 1024x768");
 let builds=ui.get_loader_version_choices();
 assert_eq!(builds.row_count(),or_page.items.len(),"Ornithe build list must load at 1024x768");
 assert!(builds.row_data(0).unwrap_or_default().to_string().starts_with(or_top.as_str()),"top build must be the live newest at 1024x768");
 let p=render(&w,1024,768);bmp(&p,"profiles-proof/picker-ornithe-1024.bmp",1024,768);
 println!("capture: profiles-proof/picker-ornithe-1024.bmp (Ornithe selected at 1024x768)");
 println!("Ornithe picker proof complete: live data, real events, exclusive selection, 2 captures.");Ok(())
}
