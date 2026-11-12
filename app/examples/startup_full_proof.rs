//! Full launcher layout fixture. No account/session/data access, no OS windows.
use slint::{platform::{software_renderer::{MinimalSoftwareWindow,RepaintBufferType},Platform,WindowAdapter},ComponentHandle,Rgb8Pixel};
use std::rc::Rc;
#[path="../src/startup.rs"] mod startup;
slint::include_modules!();
#[path="../src/theme_ui.rs"] mod theme_ui;
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
fn main()->Result<(),slint::PlatformError>{
 let w=MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);slint::platform::set_platform(Box::new(Offscreen(w.clone()))).unwrap();let ui=MainWindow::new()?;
 std::fs::create_dir_all("startup-full-proof").unwrap();
 ui.set_startup_glyphs(slint::ModelRc::new(slint::VecModel::from(startup::frame(2.1).glyphs.iter().map(|g|StartupGlyph{visible:g.visible,x:g.x as f32,y:g.y as f32,sx:g.sx as f32,sy:g.sy as f32}).collect::<Vec<_>>())));
 for (width,height) in [(1728,936),(1600,900),(1100,700),(860,620)] {
  ui.window().set_size(slint::PhysicalSize::new(width,height));ui.show()?;
  for step in 0..=20 {let sw=if width==1728{1248.}else{720.};let sh=sw*9./16.;let q=step as f32/20.;let scale=ui.get_startup_dest_width()/1120.24;let ex=ui.get_startup_dest_x()-400.28*scale;let ey=ui.get_startup_dest_y()+(28.-75.*ui.get_startup_dest_width()/836.)/2.-456.34*scale;
   let end=ui.global::<ThemePalette>().get_text_color();let mix=|v:u8|(255.+(v as f32-255.)*q).round()as u8;ui.set_startup_ink(slint::Color::from_rgb_u8(mix(end.red()),mix(end.green()),mix(end.blue())));
   ui.set_startup_overlay(true);ui.set_startup_x((width as f32-sw)/2.*(1.-q)+ex*q);ui.set_startup_y((height as f32-sh)/2.*(1.-q)+ey*q);ui.set_startup_width(sw*(1.-q)+1920.*scale*q);
   capture(&w,&format!("startup-full-proof/handoff-{width}-{step:02}.bmp"),width as usize,height as usize);
  }
  ui.set_startup_overlay(false);capture(&w,&format!("startup-full-proof/ready-{width}.bmp"),width as usize,height as usize);
 }
 let theme_timer=std::rc::Rc::new(slint::Timer::default());
 for pack in andromeda_core::themes::builtins() {
  theme_ui::apply(&ui,&pack,false,&theme_timer);
  for (width,height) in [(1728,936),(1600,900),(1100,700),(860,620)] {
   ui.window().set_size(slint::PhysicalSize::new(width,height));
   let scale=ui.get_startup_dest_width()/1120.24;let ex=ui.get_startup_dest_x()-400.28*scale;let ey=ui.get_startup_dest_y()+(28.-75.*ui.get_startup_dest_width()/836.)/2.-456.34*scale;
   ui.set_startup_ink(ui.global::<ThemePalette>().get_text_color());ui.set_startup_overlay(true);ui.set_startup_x(ex);ui.set_startup_y(ey);ui.set_startup_width(1920.*scale);
   capture(&w,&format!("startup-full-proof/theme-{}-{width}-end.bmp",pack.id),width as usize,height as usize);
   ui.set_startup_overlay(false);capture(&w,&format!("startup-full-proof/theme-{}-{width}-ready.bmp",pack.id),width as usize,height as usize);
  }
 }
 ui.window().dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged{scale_factor:1.5});
 ui.window().set_size(slint::PhysicalSize::new(1290,930));ui.set_startup_overlay(false);capture(&w,"startup-full-proof/dpi-150.bmp",1290,930);
 println!("63 offscreen handoff and3ready frames, no accounts or OS windows.");Ok(())
}
