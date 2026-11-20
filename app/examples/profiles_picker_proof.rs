//! Profiles picker proof. Renders the Settings/Profiles section with the
//! version filter chips, the version list and the one-loader toggle fully
//! offscreen. No desktop window, account or game effects.
use slint::{platform::{software_renderer::{MinimalSoftwareWindow,RepaintBufferType},Platform,WindowAdapter},ComponentHandle,Rgb8Pixel,ModelRc,VecModel,SharedString};
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
fn main()->Result<(),slint::PlatformError>{
 let w=MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);slint::platform::set_platform(Box::new(Offscreen(w.clone()))).unwrap();
 let ui=MainWindow::new()?;ui.window().set_size(slint::PhysicalSize::new(1600,900));
 let labels:Vec<SharedString>=["1.21.11 · release","1.21.10 · release","1.21.1 · release","1.20.1 · release","1.12.2 · release","b1.7.3 · old beta"].iter().map(|s|SharedString::from(*s)).collect();
 ui.set_version_choices(ModelRc::new(VecModel::from(labels)));
 ui.set_active_tab(3);ui.set_settings_section(0);
 ui.set_version_filter(0);ui.set_version_index(2);ui.set_loader_index(1);
 ui.show()?;
 std::fs::create_dir_all("profiles-proof").unwrap();
 capture(&w,"profiles-proof/picker-fabric-stable.bmp",1600,900);
 ui.set_version_filter(3);ui.set_loader_index(4);ui.set_version_index(5);
 capture(&w,"profiles-proof/picker-neoforge-all.bmp",1600,900);
 ui.set_version_filter(1);ui.set_loader_index(0);ui.set_version_index(0);
 capture(&w,"profiles-proof/picker-vanilla-snapshots.bmp",1600,900);
 println!("Profiles picker proof rendered 3 states, 1600x900. No real window opened.");Ok(())
}
