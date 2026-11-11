//! Event-loop startup orchestration. Only local preparation gates the intro.
use crate::{startup, startup_gate::{Gate,Stage,Event},startup_data, startup_cover,PreparedMain};
use slint::ComponentHandle;
use std::{cell::RefCell,rc::Rc,path::PathBuf,time::{Instant,Duration},sync::{Arc,atomic::{AtomicBool,Ordering}}};
mod ui {slint::slint! {import {SplashWindow} from "../ui/startup.slint";}}
thread_local! {static ACTIVE:RefCell<Option<Rc<RefCell<Runtime>>>>=const {RefCell::new(None)};}
struct Runtime {
 log_guard:Option<tracing_appender::non_blocking::WorkerGuard>,
 splash:ui::SplashWindow, main:Option<PreparedMain>, gate:Gate,dir:PathBuf,
 cancel:Arc<AtomicBool>, rendered:Arc<AtomicBool>, timer:slint::Timer,
 started:Instant, intro_start:Option<Instant>, handoff_start:Option<Instant>,
 origin:(f32,f32,f32),screen_origin:(i32,i32,u32), waiting_since:Option<Instant>,
}
fn with_active(f:impl FnOnce(&mut Runtime)){ACTIVE.with(|a|{let active=a.borrow().clone();if let Some(r)=active {f(&mut r.borrow_mut());}});}
pub fn run(dir:PathBuf)->Result<(),slint::PlatformError>{
 let splash=ui::SplashWindow::new()?;
 splash.set_ambience(0.65);
 splash.set_cover_mask(slint::Image::default());
 if let Some(p)=crate::startup_placement::current(){
  splash.window().set_size(slint::PhysicalSize::new(p.width,p.height));
  splash.window().set_position(slint::PhysicalPosition::new(p.x,p.y));
 }
 splash.show()?;
 let r=Rc::new(RefCell::new(Runtime {log_guard:None,splash,main:None,gate:Gate::new(false),dir,cancel:Arc::new(AtomicBool::new(false)),rendered:Arc::new(AtomicBool::new(false)),timer:slint::Timer::default(),started:Instant::now(),intro_start:None,handoff_start:None,origin:(0.,0.,720.),screen_origin:(0,0,720),waiting_since:None}));
 ACTIVE.with(|a|*a.borrow_mut()=Some(r.clone()));
 {
  let rt=r.borrow();rt.splash.on_retry(||with_active(|r|{if r.gate.retry().is_some(){r.cancel=Arc::new(AtomicBool::new(false));r.splash.set_failure("".into());r.started=Instant::now();if let Some(p)=crate::startup_placement::current(){r.splash.window().set_size(slint::PhysicalSize::new(p.width,p.height));r.splash.window().set_position(slint::PhysicalPosition::new(p.x,p.y));}r.splash.set_letters_visible(false);r.splash.set_planet_x(960.);r.splash.set_planet_y(540.);r.splash.set_planet_size(650.);r.splash.set_ambience(0.65);r.splash.set_cover_mask(slint::Image::default());r.load();}}));
  rt.splash.on_cancel(||with_active(|r|r.close()));
  rt.splash.window().on_close_requested(||{with_active(|r|r.close());slint::CloseRequestResponse::KeepWindowShown});
  rt.timer.start(slint::TimerMode::Repeated,Duration::from_millis(16),||with_active(|r|r.tick()));
 }
 // First event-loop turn draws the splash. Start disk work on the next tick.
 slint::Timer::single_shot(Duration::from_millis(1),||with_active(|r|r.load()));
 let result=slint::run_event_loop();
 r.borrow_mut().close();ACTIVE.with(|a|*a.borrow_mut()=None);result
}
impl Runtime {
 fn close(&mut self){self.cancel.store(true,Ordering::Relaxed);self.gate.close();self.timer.stop();if let Some(m)=&self.main {m.auth_cancel.store(true,Ordering::Relaxed);}let _=self.splash.hide();let _=slint::quit_event_loop();}
 fn fail(&mut self,message:String){let a=self.gate.attempt();self.gate.accept(a,Event::Fail);self.cancel.store(true,Ordering::Relaxed);self.splash.set_failure(message.into());let _=self.splash.show();if let Some(m)=self.main.take(){m.auth_cancel.store(true,Ordering::Relaxed);let _=m.ui.hide();}}
 fn load(&mut self){
  if self.gate.stage()!=Stage::Loading {return;}
  if self.log_guard.is_none() {
   let appender=match tracing_appender::rolling::Builder::new().rotation(tracing_appender::rolling::Rotation::DAILY).filename_prefix("andromeda.log").build(self.dir.join("logs")) {
    Ok(a)=>a,Err(e)=>{self.fail(format!("No se pudieron abrir los registros del launcher: {e}"));return;}
   };
   let (writer,guard)=tracing_appender::non_blocking(appender);
   let _=tracing_subscriber::fmt().with_writer(writer).with_ansi(false).try_init();self.log_guard=Some(guard);
  }
  let root=self.dir.clone();let cancel=self.cancel.clone();let attempt=self.gate.attempt();let weak=self.splash.as_weak();
  std::thread::spawn(move ||{
   let prepared=std::panic::catch_unwind(||startup_data::prepare(&root,&cancel));
   if cancel.load(Ordering::Relaxed){return;}
   let _=weak.upgrade_in_event_loop(move |_|with_active(|r|{
    if r.gate.attempt()!=attempt||r.gate.stage()!=Stage::Loading{return;}
    match prepared {
     Ok(Ok(data))=>{
      r.gate.set_reduce_motion(data.settings.reduce_animations);
      r.gate.accept(attempt,Event::DataLoaded);
      match crate::prepare_main(r.dir.clone(),data){
       Ok(main)=>{
        let flag=r.rendered.clone();
        if let Err(e)=main.ui.window().set_rendering_notifier(move |state,_|{if matches!(state,slint::RenderingState::AfterRendering){flag.store(true,Ordering::Release);}}){main.auth_cancel.store(true,Ordering::Relaxed);r.fail(format!("No se pudo verificar el primer fotograma: {e:?}"));return;}
        main.ui.set_startup_glyphs(slint::ModelRc::new(slint::VecModel::from(startup::frame(2.1).glyphs.iter().map(|g|crate::StartupGlyph{visible:g.visible,x:g.x as f32,y:g.y as f32,sx:g.sx as f32,sy:g.sy as f32}).collect::<Vec<_>>())));
        r.main=Some(main);r.gate.accept(attempt,Event::MainPrepared);
        if r.gate.stage()==Stage::Intro {r.intro_start=Some(Instant::now()+Duration::from_millis(300));}else{r.show_main();}
       },
       Err(e)=>r.fail(format!("No se pudo preparar el launcher: {e}")),
      }
     },
     Ok(Err(startup_data::PrepareError::Cancelled))=>{},
     Err(_)=>r.fail("No se pudieron leer los datos locales. Tus archivos no se han cambiado.".into()),
    }
   }));
  });
 }
 fn show_main(&mut self){
  let Some(main)=&self.main else{return;};
  self.waiting_since=Some(Instant::now());
  self.rendered.store(false,Ordering::Release);
  main.ui.set_startup_overlay(true);
  let sp=self.splash.window().position();let ss=self.splash.window().size();
  let ms=main.ui.window().size();
  main.ui.window().set_position(slint::PhysicalPosition::new(sp.x+(ss.width as i32-ms.width as i32)/2,sp.y+(ss.height as i32-ms.height as i32)/2));
  if let Err(e)=main.ui.show(){self.fail(format!("No se pudo mostrar el launcher: {e}"));return;}
  let mp=main.ui.window().position();let sf=main.ui.window().scale_factor();
  self.screen_origin=(sp.x,sp.y,ss.width);
  self.origin=((sp.x-mp.x) as f32/sf,(sp.y-mp.y) as f32/sf,ss.width as f32/sf);
  main.ui.set_startup_x(self.origin.0);main.ui.set_startup_y(self.origin.1);main.ui.set_startup_width(self.origin.2);
  main.ui.window().request_redraw();
 }
 fn tick(&mut self){
  let attempt=self.gate.attempt();
  match self.gate.stage(){
   Stage::Loading=>{let level=0.65+0.2*(self.started.elapsed().as_secs_f32()*std::f32::consts::TAU/2.8).sin();self.splash.set_ambience(level);if self.started.elapsed()>Duration::from_secs(60){self.fail("La lectura de los datos locales tarda demasiado. Puedes reintentar o cerrar sin cambiar tus archivos.".into());return;}},
   Stage::Intro=>{
    let start=self.intro_start.unwrap();
    let Some(elapsed)=Instant::now().checked_duration_since(start) else {
     let left=start.saturating_duration_since(Instant::now()).as_secs_f32()/0.3;
     let ready_t=(self.started.elapsed().as_secs_f32()-(0.3-start.saturating_duration_since(Instant::now()).as_secs_f32())).max(0.);
     let level=(0.65+0.2*(ready_t*std::f32::consts::TAU/2.8).sin())*left.clamp(0.,1.);
     self.splash.set_ambience(level);return;
    };
    self.splash.set_ambience(0.);let t=elapsed.as_secs_f64();let st=startup::frame(t);
    self.splash.set_letters_visible(true);self.splash.set_planet_x(st.x as f32);self.splash.set_planet_y(st.y as f32);self.splash.set_planet_size(st.size as f32);
    self.splash.set_cover_mask(startup_cover::cover(st.x,st.y,st.size,720,405));
    self.splash.set_glyphs(slint::ModelRc::new(slint::VecModel::from(st.glyphs.iter().map(|g|ui::StartupGlyph{visible:g.visible,x:g.x as f32,y:g.y as f32,sx:g.sx as f32,sy:g.sy as f32}).collect::<Vec<_>>())));
    if t>=startup::INTRO_SECONDS {self.gate.accept(attempt,Event::IntroFinished);self.show_main();}
   },
   Stage::AwaitingMainFrame=>{
    if self.waiting_since.is_some_and(|t|t.elapsed()>Duration::from_secs(8)){self.fail("El launcher no ha dibujado su primer fotograma. Puedes reintentar sin cambiar tus datos.".into());return;}
    if self.rendered.load(Ordering::Acquire){
     if let Some(main)=&self.main {let mp=main.ui.window().position();let sf=main.ui.window().scale_factor();let corrected=((self.screen_origin.0-mp.x)as f32/sf,(self.screen_origin.1-mp.y)as f32/sf,self.screen_origin.2 as f32/sf);
      if (corrected.0-self.origin.0).abs()>0.25||(corrected.1-self.origin.1).abs()>0.25||(corrected.2-self.origin.2).abs()>0.25 {self.origin=corrected;main.ui.set_startup_x(corrected.0);main.ui.set_startup_y(corrected.1);main.ui.set_startup_width(corrected.2);self.rendered.store(false,Ordering::Release);main.ui.window().request_redraw();return;}
     }
     self.gate.accept(attempt,Event::MainFirstFrame);let _=self.splash.hide();self.handoff_start=Some(Instant::now());if self.gate.stage()==Stage::Ready {if let Some(m)=&self.main {m.ui.set_startup_overlay(false);}}}
   },
   Stage::Handoff=>{
    let t=(self.handoff_start.unwrap().elapsed().as_secs_f32()/0.55).clamp(0.,1.);let q=t*t*t*(t*(t*6.-15.)+10.);
    if let Some(main)=&self.main {
     let mp=main.ui.window().position();let sf=main.ui.window().scale_factor();self.origin=((self.screen_origin.0-mp.x)as f32/sf,(self.screen_origin.1-mp.y)as f32/sf,self.screen_origin.2 as f32/sf);
     // Re-read the slot every tick so resize/theme/DPI changes cannot leave a stale endpoint.
     let end=main.ui.global::<crate::ThemePalette>().get_text_color();
     let mix=|v:u8|(255.+(v as f32-255.)*q).round()as u8;
     main.ui.set_startup_ink(slint::Color::from_rgb_u8(mix(end.red()),mix(end.green()),mix(end.blue())));
     let w=main.ui.get_startup_dest_width();let scale=w/1120.24;
     let endx=main.ui.get_startup_dest_x()-400.28*scale;
     let endy=main.ui.get_startup_dest_y()+(28.-75.*w/836.)/2.-456.34*scale;
     main.ui.set_startup_x(self.origin.0+(endx-self.origin.0)*q);main.ui.set_startup_y(self.origin.1+(endy-self.origin.1)*q);main.ui.set_startup_width(self.origin.2+(1920.*scale-self.origin.2)*q);
     if t>=1.{main.ui.set_startup_overlay(false);self.gate.accept(attempt,Event::HandoffFinished);}
    }
   },
   _=>{},
  }
 }
}
