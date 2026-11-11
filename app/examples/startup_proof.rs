//! Native vector-only startup proof. No desktop window, account or game effects.
use slint::{platform::{software_renderer::{MinimalSoftwareWindow,RepaintBufferType},Platform,WindowAdapter},ComponentHandle,Rgb8Pixel};
use std::rc::Rc;
#[path="../src/startup.rs"] mod startup;
#[path="../src/startup_v23_parity.rs"] mod startup_v23_parity;
#[path="../src/startup_cover.rs"] mod startup_cover;
slint::slint! { import { SplashWindow } from "../ui/startup.slint"; }
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
 let w=MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);slint::platform::set_platform(Box::new(Offscreen(w.clone()))).unwrap();let ui=SplashWindow::new()?;ui.window().set_size(slint::PhysicalSize::new(720,405));ui.show()?;
 std::fs::create_dir_all("startup-proof").unwrap();
 for f in 0..40 {ui.set_cover_mask(startup_cover::cover(960.,540.,650.,720,405));ui.set_letters_visible(false);ui.set_planet_x(960.);ui.set_planet_y(540.);ui.set_planet_size(650.);ui.set_arc_commands(startup::loader_arc(f as f64/30.).into());capture(&w,&format!("startup-proof/loader-{f:03}.bmp"),720,405);}
 for f in 0..126 {let st=startup::frame(f as f64/60.);ui.set_cover_mask(startup_cover::cover(st.x,st.y,st.size,720,405));ui.set_letters_visible(true);ui.set_planet_x(st.x as f32);ui.set_planet_y(st.y as f32);ui.set_planet_size(st.size as f32);ui.set_glyphs(slint::ModelRc::new(slint::VecModel::from(st.glyphs.iter().map(|g|StartupGlyph{visible:g.visible,x:g.x as f32,y:g.y as f32,sx:g.sx as f32,sy:g.sy as f32}).collect::<Vec<_>>())));capture(&w,&format!("startup-proof/intro-{f:03}.bmp"),720,405);}
 ui.set_failure("No se pudo preparar el launcher. Tus archivos no se han cambiado.".into());capture(&w,"startup-proof/failure.bmp",720,405);
 println!("Native startup proof rendered 40 loader and 126 intro frames, 720x405. No real window opened.");Ok(())
}
