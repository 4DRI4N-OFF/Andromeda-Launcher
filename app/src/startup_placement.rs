//! Physical-pixel splash placement. No global display settings are changed.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Placement {pub x:i32,pub y:i32,pub width:u32,pub height:u32}
#[allow(dead_code)] // used by tests, proof examples and the Windows build; not called by the Linux bin
pub fn fit(left:i32,top:i32,right:i32,bottom:i32)->Option<Placement>{
 let w=right.checked_sub(left)?;let h=bottom.checked_sub(top)?;
 if w<32||h<18{return None;}
 // 65% of work-area width, minimum960 physical pixels where space allows.
 // Never occupy more than85% of either work-area axis; keep exact16:9.
 let preferred=(w as f64*0.65).max(960.);
 let maxw=(w as f64*0.85).min(h as f64*0.85*16./9.);
 let units=(preferred.min(maxw)/16.).floor().max(1.)as u32;
 let width=units*16;let height=units*9;
 Some(Placement{x:left+(w-width as i32)/2,y:top+(h-height as i32)/2,width,height})
}
// Main window has its own work-area-relative size. It is intentionally larger
// than the splash on both axes, without requesting maximization.
#[allow(dead_code)] // used by tests, proof examples and the Windows build; not called by the Linux bin
pub fn fit_main(left:i32,top:i32,right:i32,bottom:i32)->Option<Placement>{
 let w=right.checked_sub(left)?;let h=bottom.checked_sub(top)?;
 if w<32||h<18{return None;}
 let width=(w as f64*0.90).floor()as u32;
 let height=(h as f64*0.90).floor()as u32;
 Some(Placement{x:left+(w-width as i32)/2,y:top+(h-height as i32)/2,width,height})
}
#[cfg(windows)] fn monitor_area(at:Option<(i32,i32)>)->Option<(i32,i32,i32,i32)>{
 use std::ffi::c_void;
 #[repr(C)]#[derive(Default)]struct Point{x:i32,y:i32}
 #[repr(C)]#[derive(Default)]struct Rect{left:i32,top:i32,right:i32,bottom:i32}
 #[repr(C)]struct Info{size:u32,monitor:Rect,work:Rect,flags:u32}
 #[link(name="user32")]extern "system" {
  fn SetThreadDpiAwarenessContext(value:*mut c_void)->*mut c_void;
  fn GetCursorPos(point:*mut Point)->i32;
  fn MonitorFromPoint(point:Point,flags:u32)->*mut c_void;
  fn GetMonitorInfoW(monitor:*mut c_void,info:*mut Info)->i32;
 }
 unsafe{
  // Ask for unvirtualized physical monitor coordinates, restoring Slint's context.
  let old=SetThreadDpiAwarenessContext(-4isize as *mut c_void);
  let mut point=Point::default();let cursor=if let Some((x,y))=at{point=Point{x,y};true}else{GetCursorPos(&mut point)!=0};
  let monitor=MonitorFromPoint(point,if cursor{2}else{1});
  let mut info=Info{size:std::mem::size_of::<Info>()as u32,monitor:Rect::default(),work:Rect::default(),flags:0};
  let ok=GetMonitorInfoW(monitor,&mut info)!=0;
  if !old.is_null(){SetThreadDpiAwarenessContext(old);}
  if ok{Some((info.work.left,info.work.top,info.work.right,info.work.bottom))}else{None}
 }
}
#[cfg(windows)]pub fn current()->Option<Placement>{let(l,t,r,b)=monitor_area(None)?;fit(l,t,r,b)}
#[cfg(windows)]pub fn main_at(x:i32,y:i32)->Option<Placement>{let(l,t,r,b)=monitor_area(Some((x,y)))?;fit_main(l,t,r,b)}
#[cfg(not(windows))]pub fn main_at(_x:i32,_y:i32)->Option<Placement>{None}
#[cfg(not(windows))]pub fn current()->Option<Placement>{None}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn main_1080(){let p=fit(0,0,1920,1040).unwrap();assert_eq!((p.width,p.height),(1248,702));assert_eq!((p.x,p.y),(336,169));}
 #[test]fn secondary_negative_origin(){let a=fit(-2560,-300,0,1100).unwrap();assert_eq!(a.x,-2560+(2560-a.width as i32)/2);assert_eq!(a.y,-300+(1400-a.height as i32)/2);}
 #[test]fn physical_4k(){let p=fit(0,0,3840,2080).unwrap();assert_eq!((p.width,p.height),(2496,1404));}
 #[test]fn small_and_portrait_fit(){for(w,h)in[(1024,728),(800,560),(1080,1880),(1280,680)]{let p=fit(0,0,w,h).unwrap();assert_eq!(p.width*9,p.height*16);assert!(p.width as f64<=w as f64*0.85);assert!(p.height as f64<=h as f64*0.85);}}
 #[test]fn main_is_larger_than_splash_on_both_axes(){for(l,t,r,b)in[(0,0,1920,1040),(0,0,3840,2080),(-2560,-300,0,1100),(0,0,1024,728),(0,0,800,560),(0,0,1080,1880),(0,0,1280,680)]{let splash=fit(l,t,r,b).unwrap();let main=fit_main(l,t,r,b).unwrap();assert!(main.width>splash.width);assert!(main.height>splash.height);assert!(main.x>=l&&main.y>=t);assert!(main.x+main.width as i32<=r);assert!(main.y+main.height as i32<=b);}}
 #[test]fn main_1080_size(){let p=fit_main(0,0,1920,1040).unwrap();assert_eq!((p.x,p.y,p.width,p.height),(96,52,1728,936));}
 #[test]fn main_invalid(){assert_eq!(fit_main(0,0,0,0),None);}
 #[test]fn invalid_area(){assert_eq!(fit(0,0,0,0),None);}
}
