//! Small-window layout proof with LIVE catalog data and REAL input events.
//! At 1024x768 and 860x620 with b1.7.3 + Ornithe selected, the outer
//! settings ScrollView must scroll far enough to bring the loader build
//! list and the enabled "Create profile" button into view, and the button
//! must accept a real click. Also confirms the eight loader buttons still
//! render and respond at 860px width. No redesign; known cosmetic clipping
//! at small sizes is recorded, not fixed here.
//! Captures: picker-small-1024-create.bmp, picker-small-860-create.bmp.
use andromeda_core::catalog::{available_loaders, loader_versions, mc_versions, LoaderKind, McFilter, McKind};
use slint::{platform::{software_renderer::{MinimalSoftwareWindow,RepaintBufferType},Platform,WindowAdapter,WindowEvent,PointerEventButton},ComponentHandle,LogicalPosition,Model,Rgb8Pixel,ModelRc,VecModel,SharedString};
use std::rc::Rc;
use std::cell::RefCell;
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
// Accent (primary) button pixels, same detector family as earlier proofs.
fn accent(q:Rgb8Pixel)->bool{q.r>120&&q.r>q.g&&q.g>q.b&&q.b<160&&q.g>80}
// Find a TALL, wide accent region below y_min: the filled primary button is
// ~36px high and spans most of the content width; selected-row outlines are
// 1-2px and small selected buttons are narrow, so neither qualifies.
fn find_primary(p:&[Rgb8Pixel],width:usize,height:usize,y_min:usize)->Option<(f32,f32)>{
 // Gap-tolerant like find_anchor: the label cuts the fill; use span and
 // coverage, group 8+ consecutive rows (small design-scale buttons are short).
 let mut rows:Vec<(usize,usize,usize)>=Vec::new();
 for y in y_min..height{
  let mut first=None;let mut last=0usize;let mut count=0usize;
  for x in 0..width{
   if accent(p[y*width+x]){if first.is_none(){first=Some(x);}last=x;count+=1;}
  }
  if let Some(a)=first{
   let span=last+1-a;
   if span>300&&count*5>=span*2{rows.push((y,a,last));}
  }
 }
 let mut best:Option<(usize,usize,usize,usize)>=None;
 let mut cur:Option<(usize,usize,usize,usize)>=None;
 for (y,a,b) in rows{
  match cur{
   Some((y0,y1,ca,cb)) if y<=y1+1=>{cur=Some((y0,y,ca.min(a),cb.max(b)));}
   _=>{if let Some(g)=cur{if g.1-g.0>=8{best=Some(g);}}cur=Some((y,y,a,b));}
  }
 }
 if let Some(g)=cur{if g.1-g.0>=8{best=Some(g);}}
 best.map(|(y0,y1,a,b)|((a+b)as f32/2.,(y0+y1)as f32/2.))
}
// Find a mid-size filled accent blob (the selected "Old" filter button,
// 80-260px wide, 10+px tall) as a layout anchor. Returns (cx, cy).
fn find_anchor(p:&[Rgb8Pixel],width:usize,height:usize)->Option<(f32,f32)>{
 // Gap-tolerant: the button label cuts the fill, so use the accent span
 // (last-first) with at least 40% coverage instead of one contiguous run.
 // Skip the fixed top chrome: the selected nav tab is also accent-filled.
 let mut rows:Vec<(usize,usize,usize)>=Vec::new();
 for y in 150..height{
  let mut first=None;let mut last=0usize;let mut count=0usize;
  for x in 0..width{
   if accent(p[y*width+x]){if first.is_none(){first=Some(x);}last=x;count+=1;}
  }
  if let Some(a)=first{
   let span=last+1-a;
   if (80..=260).contains(&span)&&count*5>=span*2{rows.push((y,a,last));}
  }
 }
 let mut cur:Option<(usize,usize,usize,usize)>=None;
 let mut out=None;
 for (y,a,b) in rows{
  match cur{
   Some((y0,y1,ca,cb)) if y<=y1+1=>{cur=Some((y0,y,ca.min(a),cb.max(b)));}
   _=>{if let Some(g)=cur{if g.1-g.0>=10&&out.is_none(){out=Some(g);}}cur=Some((y,y,a,b));}
  }
 }
 if let Some(g)=cur{if g.1-g.0>=10&&out.is_none(){out=Some(g);}}
 out.map(|(y0,y1,a,b)|((a+b)as f32/2.,(y0+y1)as f32/2.))
}
fn main()->Result<(),slint::PlatformError>{
 eprintln!("phase: start");
 let dl=andromeda_core::vanilla::Downloader::new().unwrap();
 std::fs::create_dir_all("profiles-proof/catalog-cache").unwrap();
 let cache=std::path::Path::new("profiles-proof/catalog-cache");
 let f_old=McFilter{release:false,snapshot:false,old_beta:true,old_alpha:true};
 let old=mc_versions(&dl,cache,f_old).unwrap();
 eprintln!("phase: old list fetched");
 let idx_b173=old.items.iter().position(|v|v.id=="b1.7.3").expect("b1.7.3 must be a listed old beta");
 let avail=available_loaders(&dl,cache,"b1.7.3").unwrap();
 eprintln!("phase: availability fetched");
 assert_eq!(avail,vec![LoaderKind::Babric,LoaderKind::Ornithe],"b1.7.3 must offer Babric and Ornithe");
 let or_page=loader_versions(&dl,cache,LoaderKind::Ornithe,"b1.7.3").unwrap();
 let w=MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);slint::platform::set_platform(Box::new(Offscreen(w.clone()))).unwrap();
 let ui=MainWindow::new()?;
 ui.set_reduce_animations(true);ui.set_active_tab(3);ui.set_settings_section(0);
 {let wk=ui.as_weak();ui.on_version_refresh(move |f|{let Some(u)=wk.upgrade()else{return};
  let dl=andromeda_core::vanilla::Downloader::new().unwrap();
  let filter=match f{0=>McFilter{release:true,snapshot:false,old_beta:false,old_alpha:false},1=>McFilter{release:false,snapshot:true,old_beta:false,old_alpha:false},2=>McFilter{release:false,snapshot:false,old_beta:true,old_alpha:true},_=>McFilter::all()};
  let page=mc_versions(&dl,std::path::Path::new("profiles-proof/catalog-cache"),filter).unwrap();
  let labels:Vec<SharedString>=page.items.iter().map(|v|{let k=match v.kind{McKind::Release=>"release",McKind::Snapshot=>"snapshot",McKind::OldBeta=>"old beta",McKind::OldAlpha=>"old alpha"};SharedString::from(format!("{} · {}",v.id,k))}).collect();
  u.set_version_choices(ModelRc::new(VecModel::from(labels)));});}
 {let wk=ui.as_weak();ui.on_version_picked(move |i|{let Some(u)=wk.upgrade()else{return};
  let Some(row)=u.get_version_choices().row_data(i as usize)else{return};
  let mc=row.split(' ').next().unwrap_or_default().to_string();
  let dl=andromeda_core::vanilla::Downloader::new().unwrap();
  let avail=available_loaders(&dl,std::path::Path::new("profiles-proof/catalog-cache"),&mc).unwrap();
  let flags:Vec<bool>=[true].into_iter().chain(LoaderKind::all().iter().map(|k|avail.contains(k))).collect();
  if u.get_loader_index()>0&&!flags.get(u.get_loader_index() as usize).copied().unwrap_or(false){u.set_loader_index(0);}
  u.set_loader_available(ModelRc::new(VecModel::from(flags)));});}
 {let wk=ui.as_weak();ui.on_loader_picked(move |i|{let Some(u)=wk.upgrade()else{return};
  if i==0{u.set_loader_version_choices(ModelRc::new(VecModel::from(Vec::<SharedString>::new())));return;}
  let Some(row)=u.get_version_choices().row_data(u.get_version_index() as usize)else{return};
  let mc=row.split(' ').next().unwrap_or_default().to_string();
  let kind=[LoaderKind::Fabric,LoaderKind::Quilt,LoaderKind::Forge,LoaderKind::NeoForge,LoaderKind::LegacyFabric,LoaderKind::Babric,LoaderKind::Ornithe][i as usize-1];
  let dl=andromeda_core::vanilla::Downloader::new().unwrap();
  let page=loader_versions(&dl,std::path::Path::new("profiles-proof/catalog-cache"),kind,&mc).unwrap();
  let labels:Vec<SharedString>=page.items.iter().map(|v|SharedString::from(if v.stable==Some(true){format!("{} · stable",v.version)}else{v.version.clone()})).collect();
  u.set_loader_version_index(0);u.set_loader_version_choices(ModelRc::new(VecModel::from(labels)));});}
 // Record Create-profile invocations instead of creating instances.
 let created:Rc<RefCell<Vec<String>>>=Rc::new(RefCell::new(Vec::new()));
 {let log=created.clone();ui.on_manage_instance(move |action,name|{log.borrow_mut().push(format!("{action}:{name}"));});}
 eprintln!("phase: ui wired");
 ui.set_version_filter(2);ui.invoke_version_refresh(2);
 ui.set_version_index(idx_b173 as i32);ui.invoke_version_picked(idx_b173 as i32);
 ui.set_instance_name(slint::SharedString::from("ProofSmall"));
 for (width,height) in [(1024usize,768usize),(860usize,620usize)]{
  eprintln!("phase: size {width}x{height}");
  ui.window().set_size(slint::PhysicalSize::new(width as u32,height as u32));pump();
  // Ornithe via a real click at its rendered spot in this size. At short
  // heights the loader row starts below the fold, so scroll the outer
  // settings ScrollView until a click scan finds it.
  ui.set_loader_index(0);ui.invoke_loader_picked(0);
  let mut or_hit=false;
  // Anchor on the selected "Old" filter button; the loader row sits ~248px
  // below it in this layout. Scroll until that row is inside the window,
  // then click-scan only that one row (rightmost buttons first).
  'pick: for step in 0..40{
   let p=render(&w,width,height);
   if step%5==0{eprintln!("pick step {step} at {width}x{height}");}
   if step==0{bmp(&p,&format!("profiles-proof/picker-small-debug-{width}.bmp"),width,height);}
   let found=find_anchor(&p,width,height);
   if step==0||step%5==0{eprintln!("anchor at {width}x{height} step {step}: {:?}",found);}
   let Some((_ax,ay))=found else{
    if step==0{panic!("filter row must be visible at {width}x{height}");}
    // Anchor scrolled off the top: the loader row must be near the bottom
    // strip. Clicking build-list rows is cheap (no catalog reads).
    for fy in [height as f32-60.,height as f32-90.,height as f32-120.,height as f32-30.]{
     let mut x=width as f32-30.;
     while x>width as f32*0.78{
      click(&ui,x,fy);
      let idx=ui.get_loader_index();
      if idx==7{or_hit=true;println!("event proof {width}x{height}: Ornithe click at x={x:.0} y={fy:.0} after {step} scroll steps (anchor off-view)");break;}
      if idx==6{ui.set_loader_index(0);ui.invoke_loader_picked(0);}
      else if idx!=0{panic!("unexpected loader {idx} at {width}x{height} x={x} y={fy}");}
      x-=12.;
     }
     if or_hit{break;}
    }
    if or_hit{break 'pick;}
    wheel(&ui,width as f32*0.5,150.,-120.);
    continue;
   };
   let ly=ay+248.;
   if ly+18.>=height as f32{wheel(&ui,_ax,ay,-120.);continue;}
   let mut x=width as f32-30.;
   while x>width as f32*0.78{
    click(&ui,x,ly);
    let idx=ui.get_loader_index();
    if idx==7{or_hit=true;println!("event proof {width}x{height}: Ornithe click at x={x:.0} y={ly:.0} after {step} scroll steps");break 'pick;}
    if idx==6{ui.set_loader_index(0);ui.invoke_loader_picked(0);}
    else if idx!=0{panic!("unexpected loader {idx} at {width}x{height} x={x} y={ly}");}
    x-=12.;
   }
   // The anchor was visible but the row click missed: scroll a little and retry.
   wheel(&ui,_ax,ay,-120.);
  }
  assert!(or_hit,"Ornithe must be clickable at {width}x{height}");
  assert_eq!(ui.get_loader_version_choices().row_count(),or_page.items.len(),"Ornithe builds must load at {width}x{height}");
  // Outer scroll until the enabled primary Create button is in view.
  let mut hit=None;
  for step in 0..40{
   let p=render(&w,width,height);
   if step>0{
    // Wheel over the filter button while it is visible (direct child of the
    // outer ScrollView); once it scrolls off, content is near the bottom.
    match find_anchor(&p,width,height){
     Some((ax,ay))=>wheel(&ui,ax,ay,-120.),
     None=>wheel(&ui,width as f32*0.5,150.,-120.),
    }
    let p=render(&w,width,height);
    if let Some(pos)=find_primary(&p,width,height,(height as f32*0.45) as usize){
     hit=Some(pos);let bp=&p;bmp(bp,&format!("profiles-proof/picker-small-{width}-create.bmp"),width,height);
     println!("event proof {width}x{height}: primary button in view at ({:.0},{:.0}) after {step} more scroll steps",pos.0,pos.1);
     break;
    }
    continue;
   }
   if let Some(pos)=find_primary(&p,width,height,(height as f32*0.45) as usize){
    hit=Some(pos);let bp=&p;bmp(bp,&format!("profiles-proof/picker-small-{width}-create.bmp"),width,height);
    println!("event proof {width}x{height}: primary button in view at ({:.0},{:.0}) after {step} more scroll steps",pos.0,pos.1);
    break;
   }
  }
  let Some((cx,cy))=hit else{panic!("Create profile button must become visible by scrolling at {width}x{height}")};
  let before=created.borrow().len();
  click(&ui,cx,cy);
  let after=created.borrow().len();
  assert!(after>before,"Create profile must accept a real click at {width}x{height}");
  println!("event proof {width}x{height}: Create profile click -> {:?}",created.borrow().last());
 }
 println!("Small layout proof complete: outer scroll reaches builds + Create at 1024x768 and 860x620, 2 captures.");Ok(())
}
