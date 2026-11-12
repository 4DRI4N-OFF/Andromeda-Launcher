//! Optional preview runner. Not production launcher wiring. Run manually only.
use slint::ComponentHandle;
use std::time::{Duration,Instant};
#[path="../src/player_mesh.rs"] mod player_mesh;
slint::slint!{ export {LobbyWindow} from "../ui/lobby.slint"; }
fn main()->Result<(),slint::PlatformError>{
 let ui=LobbyWindow::new()?;
 let pixels=player_mesh::render(include_bytes!("../assets/player-demo/fixture.rgba"),false,0.4,440,720);
 ui.set_player_render(slint::Image::from_rgba8(slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&pixels,440,720)));
 let weak=ui.as_weak();let start=Instant::now();
 let timer=slint::Timer::default();
 timer.start(slint::TimerMode::Repeated,Duration::from_millis(33),move||{
  if let Some(ui)=weak.upgrade(){
   if ui.get_active_tab()!=0{return;}
   let t=if ui.get_reduce_motion(){0.}else{start.elapsed().as_secs_f32()};
   let yaw=player_mesh::idle_yaw(t);
   let pixels=player_mesh::render_idle(include_bytes!("../assets/player-demo/fixture.rgba"),false,yaw,t,440,720);
   ui.set_player_render(slint::Image::from_rgba8(slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&pixels,440,720)));
  }
 });
 ui.run()
}
