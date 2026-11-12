//! Deterministic actual Slint UI proof. No native windows, accounts or launch.
use slint::{platform::{software_renderer::{MinimalSoftwareWindow,RepaintBufferType},Platform,WindowAdapter},ComponentHandle,Rgb8Pixel};
use std::rc::Rc;
#[path="../src/startup.rs"]mod startup;
#[path="../src/startup_cover.rs"]mod startup_cover;
#[path="../src/startup_sheen.rs"]mod startup_sheen;
#[path="../src/startup_placement.rs"]mod startup_placement;
#[path="../src/startup_resize.rs"]mod startup_resize;
slint::include_modules!();
mod splash {slint::slint!{export {SplashWindow} from "../ui/startup.slint";}}
struct Offscreen(Rc<MinimalSoftwareWindow>);
impl Platform for Offscreen{fn create_window_adapter(&self)->Result<Rc<dyn WindowAdapter>,slint::PlatformError>{Ok(self.0.clone())}}
fn capture(w:&MinimalSoftwareWindow,path:&str,width:usize,height:usize){
 use std::io::Write;
 w.request_redraw(); let mut pixels=vec![Rgb8Pixel::default();width*height];
 w.draw_if_needed(|r|{r.render(&mut pixels,width);});
 let stride=(width*3+3)&!3;let mut f=std::fs::File::create(path).unwrap();let mut h=vec![0u8;54];
 h[0..2].copy_from_slice(b"BM");h[2..6].copy_from_slice(&((54+stride*height)as u32).to_le_bytes());h[10..14].copy_from_slice(&54u32.to_le_bytes());h[14..18].copy_from_slice(&40u32.to_le_bytes());h[18..22].copy_from_slice(&(width as i32).to_le_bytes());h[22..26].copy_from_slice(&(height as i32).to_le_bytes());h[26..28].copy_from_slice(&1u16.to_le_bytes());h[28..30].copy_from_slice(&24u16.to_le_bytes());f.write_all(&h).unwrap();
 for y in (0..height).rev(){let mut row=vec![0u8;stride];for x in 0..width{let p=pixels[y*width+x];row[x*3..x*3+3].copy_from_slice(&[p.b,p.g,p.r]);}f.write_all(&row).unwrap();}
}
fn main()->Result<(),slint::PlatformError>{
 let w=MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);slint::platform::set_platform(Box::new(Offscreen(w.clone()))).unwrap();
 std::fs::create_dir_all("resize-proof").unwrap();
 let splash=splash::SplashWindow::new()?;let mut main:Option<MainWindow>=None;
 let end=startup::frame(2.1);
 splash.set_letters_visible(true);splash.set_glyphs(slint::ModelRc::new(slint::VecModel::from(end.glyphs.iter().map(|g|splash::StartupGlyph{visible:g.visible,x:g.x as f32,y:g.y as f32,sx:g.sx as f32,sy:g.sy as f32}).collect::<Vec<_>>())));
 splash.set_planet_x(end.x as f32);splash.set_planet_y(end.y as f32);splash.set_planet_size(end.size as f32);splash.set_locked_logo_width(1248.);

 let motion=startup_resize::Resize{from:startup_placement::fit(0,0,1920,1040).unwrap(),to:startup_placement::fit_main(0,0,1920,1040).unwrap()};
 for f in 0..168{
  let time=f as f32/30.;
  let growth=startup_sheen::END as f32;let handoff=growth+startup_resize::SECONDS;
  if time<handoff{
   let st=startup::frame(time as f64);splash.set_planet_x(st.x as f32);splash.set_planet_y(st.y as f32);splash.set_planet_size(st.size as f32);
   splash.set_glyphs(slint::ModelRc::new(slint::VecModel::from(st.glyphs.iter().map(|g|splash::StartupGlyph{visible:g.visible,x:g.x as f32,y:g.y as f32,sx:g.sx as f32,sy:g.sy as f32}).collect::<Vec<_>>())));
   splash.set_cover_mask(startup_cover::cover(st.x,st.y,st.size,720,405));splash.set_sheen(startup_sheen::position(time as f64,false));
   splash.set_locked_logo_width(if time>=growth{1248.}else{0.});
   let p=motion.at((time-growth)/startup_resize::SECONDS);splash.window().set_size(slint::PhysicalSize::new(p.width,p.height));splash.show()?;
   capture(&w,&format!("resize-proof/frame-{f:03}.bmp"),p.width as usize,p.height as usize);
  }else{
   if main.is_none(){let ui=MainWindow::new()?; ui.set_startup_glyphs(slint::ModelRc::new(slint::VecModel::from(end.glyphs.iter().map(|g|StartupGlyph{visible:g.visible,x:g.x as f32,y:g.y as f32,sx:g.sx as f32,sy:g.sy as f32}).collect::<Vec<_>>())));
main=Some(ui);}
   let ui=main.as_ref().unwrap();
   ui.window().set_size(slint::PhysicalSize::new(1728,936));ui.show()?;
   let t=((time-handoff)/0.55).clamp(0.,1.);let q=startup_resize::ease(t);let scale=ui.get_startup_dest_width()/1120.24;
   let ex=ui.get_startup_dest_x()-400.28*scale;let ey=ui.get_startup_dest_y()+(28.-75.*ui.get_startup_dest_width()/836.)/2.-456.34*scale;
   ui.set_startup_overlay(t<1.);ui.set_startup_reveal(startup_resize::ease((t/0.65).min(1.)));
   ui.set_startup_x(240.+(ex-240.)*q);ui.set_startup_y(117.+(ey-117.)*q);ui.set_startup_width(1248.+(1920.*scale-1248.)*q);
   let end=ui.global::<ThemePalette>().get_text_color();let mix=|v:u8|(255.+(v as f32-255.)*q).round()as u8;ui.set_startup_ink(slint::Color::from_rgb_u8(mix(end.red()),mix(end.green()),mix(end.blue())));
   capture(&w,&format!("resize-proof/frame-{f:03}.bmp"),1728,936);
  }
 }
 println!("168 actual Slint offscreen frames; native resize/DWM not tested");Ok(())
}
