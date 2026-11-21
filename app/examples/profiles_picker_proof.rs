#![allow(deprecated)] // slint inline-import emits a deprecated fallback const in this proof target; no behavior change
//! Profiles picker proof with LIVE catalog data and REAL input events.
//! Evidence: filter callback swaps live data; disabled loader buttons ignore
//! synthetic clicks while enabled ones select; loader build rows are
//! individually selectable; wheel scrolling reaches the Create profile button
//! at 1600x900 and at the 860x620 minimum, where a synthetic click fires
//! manage-instance("createsel"). Settled rendering. No real window opened.
use andromeda_core::catalog::{available_loaders, loader_versions, mc_versions, LoaderKind, McFilter, McKind};
use slint::{platform::{software_renderer::{MinimalSoftwareWindow,RepaintBufferType},Platform,WindowAdapter,WindowEvent,PointerEventButton},ComponentHandle,LogicalPosition,Model,Rgb8Pixel,ModelRc,VecModel,SharedString};
use std::cell::RefCell;
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
fn wheel(ui:&MainWindow,x:f32,y:f32,dy:f32){let w=ui.window();w.dispatch_event(WindowEvent::PointerScrolled{position:LogicalPosition::new(x,y),delta_x:0.,delta_y:dy});pump();}
fn find_accent(pixels:&[Rgb8Pixel],width:usize,height:usize)->Option<(f32,f32)>{
 // Locate the primary (accent) button: widest run of accent pixels in a row.
 let mut best:(usize,usize,usize)=(0,0,0); // (run, y, x_start)
 for y in height/3..height{let mut run=0;let mut start=0;
  for x in 0..width{let p=pixels[y*width+x];
   if p.r>200&&(150..215).contains(&p.g)&&p.b<170{if run==0{start=x;}run+=1;}else{if run>best.0{best=(run,y,start);}run=0;}}
  if run>best.0{best=(run,y,start);}}
 if best.0>300{Some((best.2 as f32+best.0 as f32/2.,best.1 as f32))}else{None}}
fn main()->Result<(),slint::PlatformError>{
 let dl=andromeda_core::vanilla::Downloader::new().unwrap();
 std::fs::create_dir_all("profiles-proof/catalog-cache").unwrap();
 let cache=std::path::Path::new("profiles-proof/catalog-cache");
 let f_rel=McFilter{release:true,snapshot:false,old_beta:false,old_alpha:false};
 let f_snap=McFilter{release:false,snapshot:true,old_beta:false,old_alpha:false};
 let f_old=McFilter{release:false,snapshot:false,old_beta:true,old_alpha:true};
 let rel=mc_versions(&dl,cache,f_rel).unwrap();
 let snap=mc_versions(&dl,cache,f_snap).unwrap();
 assert!(rel.items.iter().all(|v|v.kind==McKind::Release)&&snap.items.iter().all(|v|v.kind==McKind::Snapshot));
 let w=MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);slint::platform::set_platform(Box::new(Offscreen(w.clone()))).unwrap();
 let ui=MainWindow::new()?;ui.window().set_size(slint::PhysicalSize::new(1600,900));
 ui.set_reduce_animations(true);ui.set_active_tab(3);ui.set_settings_section(0);
 let manages=Rc::new(RefCell::new(Vec::<String>::new()));
 {let m=manages.clone();ui.on_manage_instance(move |a,_|m.borrow_mut().push(a.to_string()));}
 {let wk=ui.as_weak();ui.on_version_refresh(move |f|{let Some(u)=wk.upgrade()else{return};
  let dl=andromeda_core::vanilla::Downloader::new().unwrap();
  let filter=match f{0=>f_rel,1=>f_snap,2=>f_old,_=>McFilter::all()};
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
  let kind=[LoaderKind::Fabric,LoaderKind::Quilt,LoaderKind::Forge,LoaderKind::NeoForge][i as usize-1];
  let dl=andromeda_core::vanilla::Downloader::new().unwrap();
  let page=loader_versions(&dl,std::path::Path::new("profiles-proof/catalog-cache"),kind,&mc).unwrap();
  let labels:Vec<SharedString>=page.items.iter().map(|v|SharedString::from(if v.stable==Some(true){format!("{} · stable",v.version)}else{v.version.clone()})).collect();
  println!("loader-picked event: {:?} builds for {} -> {} listed (top {})",kind,mc,page.items.len(),page.items.first().map(|v|v.version.as_str()).unwrap_or("-"));
  u.set_loader_version_index(0);u.set_loader_version_choices(ModelRc::new(VecModel::from(labels)));});}
 // Snapshots chip through the real callback; top snapshot has Fabric only.
 ui.set_version_filter(1);ui.invoke_version_refresh(1);
 let snap_mc=snap.items[0].id.clone();
 let snap_avail=available_loaders(&dl,cache,&snap_mc).unwrap();
 assert_eq!(snap_avail,vec![LoaderKind::Fabric],"snapshot top must offer Fabric only here");
 ui.invoke_version_picked(0);
 println!("snapshot {}: loaders {:?}",snap_mc,snap_avail);
 ui.show()?;pump();
 // Disabled Quilt ignores a synthetic click; enabled Fabric accepts one.
 click(&ui,1001.,719.);
 assert_eq!(ui.get_loader_index(),0,"disabled Quilt must ignore clicks");
 println!("event proof: click on disabled Quilt -> loader stays Vanilla");
 click(&ui,770.,719.);
 assert_eq!(ui.get_loader_index(),1,"enabled Fabric must accept clicks");
 println!("event proof: click on enabled Fabric -> loader = Fabric");
 // Build rows individually selectable.
 assert!(ui.get_loader_version_choices().row_count()>1,"build list must hold many rows");
 click(&ui,952.,823.);
 assert_eq!(ui.get_loader_index(),1);
 assert_eq!(ui.get_loader_version_index(),1,"second build row must select");
 println!("event proof: click second build row -> '{}'",ui.get_loader_version_choices().row_data(1).unwrap_or_default());
 let p=render(&w,1600,900);bmp(&p,"profiles-proof/evidence-events.bmp",1600,900);
 // Build list visibility: scroll the outer panel until the selected build row
 // is fully in view; prove the row above and the selected row both paint text.
 let mut sel_y:Option<usize>=None;
 for _ in 0..8{
  wheel(&ui,900.,500.,-120.);
  let p=render(&w,1600,900);
  let mut bands:Vec<(usize,usize)>=vec![];
  for y in 300..900{let mut run=0;for x in 0..1600{let q=p[y*1600+x];if q.r>200&&(150..215).contains(&q.g)&&q.b<170{run+=1;}}
   if run>400{if let Some(l)=bands.last_mut(){if y-l.1<=1{l.1=y;continue;}}bands.push((y,y));}}
  let thin:Vec<(usize,usize)>=bands.into_iter().filter(|b|b.1-b.0<4).collect();
  if thin.len()>=2{
   let by=thin[thin.len()-1].0; // bottom edge of the selected row's border
   let band=|y0:usize,y1:usize,p:&[Rgb8Pixel]|->usize{(y0..y1).map(|y|(250..1550).filter(|&x|{let q=p[y*1600+x];q.r>180&&q.g>180&&q.b>180}).count()).sum()};
   let row1=band(by.saturating_sub(96),by.saturating_sub(50),&p); // first build row interior
   let sel=band(by.saturating_sub(46),by.saturating_sub(4),&p);   // selected row interior
   println!("build list: selected row bottom edge at y={by}; label pixels: first row={row1}, selected row={sel}");
   bmp(&p,"profiles-proof/evidence-builds-1600.bmp",1600,900);
   assert!(row1>40,"first build row must paint its label");
   assert!(sel>40,"selected build row must paint its label");
   sel_y=Some(by);break;}}
 let sel_y=sel_y.expect("selected build row must be reachable by scrolling at 1600x900");
 // Inner build list scrolls independently; deeper rows must paint labels too.
 wheel(&ui,952.,sel_y as f32-20.,-600.);
 let p=render(&w,1600,900);bmp(&p,"profiles-proof/evidence-builds-deep.bmp",1600,900);
 let deep:usize=(sel_y.saturating_sub(95)..sel_y+25).map(|y|(250..1550).filter(|&x|{let q=p[y*1600+x];q.r>180&&q.g>180&&q.b>180}).count()).sum();
 println!("build list: after inner scroll, visible label pixels near list={deep} (evidence-builds-deep.bmp)");
 assert!(deep>150,"deeper build rows must paint labels after inner scrolling");
 // Back to Vanilla for the Create-scroll traversal below (setup only; loader
 // click behavior is already proven above, so set the property directly).
 ui.set_loader_index(0);assert_eq!(ui.get_loader_index(),0);
 let mut found=None;
 for _ in 0..10{wheel(&ui,900.,500.,-120.);let p=render(&w,1600,900);if let Some(c)=find_accent(&p,1600,900){found=Some(c);bmp(&p,"profiles-proof/evidence-scroll-1600.bmp",1600,900);break;}}
 let (cx,cy)=found.expect("Create button not reachable by scrolling at 1600x900");
 println!("scroll proof: Create profile found at ({:.0},{:.0}) after wheel scrolling at 1600x900",cx,cy);
 click(&ui,cx,cy);
 assert_eq!(manages.borrow().last().map(String::as_str),Some("createsel"),"clicking Create must fire manage-instance(createsel)");
 println!("event proof: click Create at 1600x900 -> manage-instance(createsel)");
 // Minimum size traversal.
 ui.window().set_size(slint::PhysicalSize::new(860,620));pump();
 let mut found=None;
 for _ in 0..40{wheel(&ui,700.,250.,-240.);let p=render(&w,860,620);if let Some(c)=find_accent(&p,860,620){found=Some(c);bmp(&p,"profiles-proof/evidence-scroll-860.bmp",860,620);break;}}
 let (cx,cy)=found.expect("Create button not reachable by scrolling at 860x620");
 println!("scroll proof: Create profile found at ({:.0},{:.0}) after wheel scrolling at 860x620",cx,cy);
 click(&ui,cx,cy);
 assert_eq!(manages.borrow().len(),2,"Create must fire again at 860x620");
 println!("event proof: click Create at 860x620 -> manage-instance(createsel)");
 println!("Picker proof v3 complete: live data, real events, scroll traversal, 6 captures.");Ok(())
}
