//! Full launcher layout fixture. No account/session/data access, no OS windows.
use slint::{platform::{software_renderer::{MinimalSoftwareWindow,RepaintBufferType},Platform,WindowAdapter},ComponentHandle,Rgb8Pixel};
use std::{rc::Rc,cell::Cell,time::Duration};
thread_local!{static CLOCK:Cell<u64>=const{Cell::new(0)};}
#[path="../src/player_mesh.rs"] mod player_mesh;
slint::slint!{ export {LobbyWindow} from "../ui/lobby.slint"; }
struct Offscreen(Rc<MinimalSoftwareWindow>);
impl Platform for Offscreen {fn create_window_adapter(&self)->Result<Rc<dyn WindowAdapter>,slint::PlatformError>{Ok(self.0.clone())} fn duration_since_start(&self)->Duration{Duration::from_millis(CLOCK.with(|c|c.get()))}}
fn capture(w:&MinimalSoftwareWindow,path:&str,width:usize,height:usize){
 use std::io::Write;
 w.request_redraw(); let mut pixels=vec![Rgb8Pixel::default();width*height];
 w.draw_if_needed(|r|{r.render(&mut pixels,width);});
 let stride=(width*3+3)&!3;let mut f=std::fs::File::create(path).unwrap();let mut h=vec![0u8;54];
 h[0..2].copy_from_slice(b"BM");h[2..6].copy_from_slice(&((54+stride*height)as u32).to_le_bytes());h[10..14].copy_from_slice(&54u32.to_le_bytes());h[14..18].copy_from_slice(&40u32.to_le_bytes());h[18..22].copy_from_slice(&(width as i32).to_le_bytes());h[22..26].copy_from_slice(&(height as i32).to_le_bytes());h[26..28].copy_from_slice(&1u16.to_le_bytes());h[28..30].copy_from_slice(&24u16.to_le_bytes());f.write_all(&h).unwrap();
 for y in (0..height).rev(){let mut row=vec![0u8;stride];for x in 0..width{let p=pixels[y*width+x];row[x*3..x*3+3].copy_from_slice(&[p.b,p.g,p.r]);}f.write_all(&row).unwrap();}
}
fn main()->Result<(),slint::PlatformError>{
 let w=MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);slint::platform::set_platform(Box::new(Offscreen(w.clone()))).unwrap();let ui=LobbyWindow::new()?;
 std::fs::create_dir_all("lobby-proof").unwrap();ui.window().set_size(slint::PhysicalSize::new(1728,936));ui.show()?;
 let tex=include_bytes!("../assets/player-demo/fixture.rgba");
 assert_eq!(ui.get_active_tab(),0);
 ui.set_selected_instance(2);assert_eq!(ui.get_selected_instance(),2);ui.set_selected_instance(0);
 for f in 0..180 {
  CLOCK.with(|c|c.set(f*1000/30));
  slint::platform::update_timers_and_animations();
  if !std::env::args().any(|a|a=="--idle-only") && f==45{ui.set_active_tab(4);}
  if !std::env::args().any(|a|a=="--idle-only") && f==105{ui.set_active_tab(0);}
  let time=f as f32/30.;
  let yaw=player_mesh::idle_yaw(time);
  let pixels=player_mesh::render_idle(tex,false,yaw,time,440,720);
  ui.set_player_render(slint::Image::from_rgba8(slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&pixels,440,720)));
  capture(&w,&format!("lobby-proof/lobby-{f:03}.bmp"),1728,936);
 }
 ui.set_reduce_motion(true);ui.set_active_tab(0);slint::platform::update_timers_and_animations();
 for(width,height) in [(1280,720),(860,620)] {
  ui.window().set_size(slint::PhysicalSize::new(width,height));
  capture(&w,&format!("lobby-proof/size-{width}.bmp"),width as usize,height as usize);
 }
 ui.window().set_size(slint::PhysicalSize::new(1728,936));ui.set_neutral(true);
 let mut tex=vec![136u8;64*64*4];for p in tex.chunks_exact_mut(4){p[3]=255;}
 let pixels=player_mesh::render(&tex,false,0.4,440,720);
 ui.set_player_render(slint::Image::from_rgba8(slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&pixels,440,720)));
 capture(&w,"lobby-proof/offline.bmp",1728,936);
 ui.set_active_tab(4);capture(&w,"lobby-proof/account-placeholder.bmp",1728,936);
 println!("180 crossfade and idle fixture frames, 3 window sizes, offline and account placeholder. No desktop, account or network.");Ok(())
}
