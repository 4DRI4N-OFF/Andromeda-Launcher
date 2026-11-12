//! Dependency-free textured triangle renderer. Real cuboids, depth buffer and
//! perspective camera. Prototype only: no network, accounts or launcher hooks.
#[derive(Clone,Copy)]struct V{x:f32,y:f32,z:f32,u:f32,v:f32}
#[derive(Clone,Copy)]struct P{x:f32,y:f32,iz:f32,u:f32,v:f32}
// Steady-state solution of a unit-mass damped spring following a sine target.
// x'' + 2*zeta*w0*x' + w0*w0*x = w0*w0*sin(w*t).
// Analytic evaluation makes previews independent of frame rate, without startup jumps.
fn spring_wave(t:f32,period:f32)->f32{
 let w=std::f32::consts::TAU/period;let w0=8.0f32;let zeta=0.85;
 let a=w0*w0-w*w;let b=2.*zeta*w0*w;
 let gain=w0*w0/(a*a+b*b).sqrt();let lag=b.atan2(a);
 gain*(w*t-lag).sin()
}
pub fn idle_yaw(time:f32)->f32{0.4+0.035*spring_wave(time,6.)}
pub fn render(texture:&[u8],slim:bool,yaw:f32,width:usize,height:usize)->Vec<u8>{
 render_idle(texture,slim,yaw,0.,width,height)
}
pub fn render_idle(texture:&[u8],slim:bool,yaw:f32,time:f32,width:usize,height:usize)->Vec<u8>{
 assert_eq!(texture.len(),64*64*4);assert!(width>0&&height>0&&width<=1024&&height<=1024);
 let mut out=vec![0;width*height*4];let mut depth=vec![0f32;width*height];
 let aw=if slim{3.}else{4.};
 // x,y,z are model units, front is +Z. Six separately articulated solids.
 let boxes=[(0.,28.,0.,8.,8.,8.,0.,0.),(0.,18.,0.,8.,12.,4.,16.,16.),
 (-(4.+aw/2.),18.,0.,aw,12.,4.,40.,16.),(4.+aw/2.,18.,0.,aw,12.,4.,32.,48.),
 (-2.,6.,0.,4.,12.,4.,0.,16.),(2.,6.,0.,4.,12.,4.,16.,48.)];
 for (part,(cx,cy,cz,w,h,d,u,v)) in boxes.into_iter().enumerate(){
  let x0=cx-w/2.;let x1=cx+w/2.;let y0=cy-h/2.;let y1=cy+h/2.;let z0=cz-d/2.;let z1=cz+d/2.;
  // UV rectangles: right, front, left, back, top, bottom.
  let faces=[([(x0,y1,z1),(x1,y1,z1),(x1,y0,z1),(x0,y0,z1)],(u+d,v+d,w,h),1.),
  ([(x1,y1,z0),(x0,y1,z0),(x0,y0,z0),(x1,y0,z0)],(u+2.*d+w,v+d,w,h),0.66),
  ([(x0,y1,z0),(x0,y1,z1),(x0,y0,z1),(x0,y0,z0)],(u,v+d,d,h),0.72),
  ([(x1,y1,z1),(x1,y1,z0),(x1,y0,z0),(x1,y0,z1)],(u+d+w,v+d,d,h),0.84),
  ([(x0,y1,z0),(x1,y1,z0),(x1,y1,z1),(x0,y1,z1)],(u+d,v,w,d),1.08),
  ([(x0,y0,z1),(x1,y0,z1),(x1,y0,z0),(x0,y0,z0)],(u+d+w,v,w,d),0.6)];
  for (vertices,(u,v,uw,vh),shade) in faces{
   let uv=[(u,v),(u+uw,v),(u+uw,v+vh),(u,v+vh)];
   let points:Vec<P>=vertices.iter().zip(uv).map(|(&(x,y,z),(u,v))|{
    let breath=spring_wave(time,4.);
    let angle=match part{2=>-0.10f32-0.018*breath,3=>0.10+0.018*breath,4=>-0.035,5=>0.035,_=>0.};
    let pivot=if part>=4{12.}else{24.};let(sa,ca)=angle.sin_cos();
    let dx=x-cx;let dy=y-pivot;
    let(px,pz)=if part==0{let(sh,ch)=(0.025*spring_wave(time,7.)).sin_cos();((x-cx)*ch+(z-cz)*sh+cx,-(x-cx)*sh+(z-cz)*ch+cz)}else{(cx+dx*ca-dy*sa,z)};
    project(V{x:px,y:pivot+dx*sa+dy*ca+if part<4{0.065*breath}else{0.},z:pz,u,v},yaw,width,height)
   }).collect();
   tri([points[0],points[1],points[2]],shade,texture,width,height,&mut out,&mut depth);
   tri([points[0],points[2],points[3]],shade,texture,width,height,&mut out,&mut depth);
  }
 }
 out
}
fn project(p:V,yaw:f32,w:usize,h:usize)->P{
 let(s,c)=yaw.sin_cos();let x=p.x*c+p.z*s;let z=-p.x*s+p.z*c;
 let pitch=0.12f32;let(sp,cp)=pitch.sin_cos();let y=(p.y-16.)*cp-z*sp;let z=(p.y-16.)*sp+z*cp;
 let iz=1./(66.-z);let f=h as f32*1.72;
 P{x:w as f32/2.+x*f*iz,y:h as f32/2.-y*f*iz,iz,u:p.u*iz,v:p.v*iz}
}
fn edge(a:P,b:P,x:f32,y:f32)->f32{(x-a.x)*(b.y-a.y)-(y-a.y)*(b.x-a.x)}
fn tri(p:[P;3],shade:f32,tex:&[u8],w:usize,h:usize,out:&mut[u8],depth:&mut[f32]){
 let area=edge(p[0],p[1],p[2].x,p[2].y);if area.abs()<0.00001{return;}
 let x0=p.iter().map(|p|p.x).fold(f32::INFINITY,f32::min).floor().max(0.)as usize;
 let x1=p.iter().map(|p|p.x).fold(f32::NEG_INFINITY,f32::max).ceil().min(w as f32)as usize;
 let y0=p.iter().map(|p|p.y).fold(f32::INFINITY,f32::min).floor().max(0.)as usize;
 let y1=p.iter().map(|p|p.y).fold(f32::NEG_INFINITY,f32::max).ceil().min(h as f32)as usize;
 for y in y0..y1{for x in x0..x1{
  let a=edge(p[1],p[2],x as f32+0.5,y as f32+0.5)/area;let b=edge(p[2],p[0],x as f32+0.5,y as f32+0.5)/area;let c=1.-a-b;
  if a<0.||b<0.||c<0.{continue;}let iz=a*p[0].iz+b*p[1].iz+c*p[2].iz;let idx=y*w+x;if iz<=depth[idx]{continue;}
  let u=((a*p[0].u+b*p[1].u+c*p[2].u)/iz).floor().clamp(0.,63.)as usize;
  let v=((a*p[0].v+b*p[1].v+c*p[2].v)/iz).floor().clamp(0.,63.)as usize;let t=(v*64+u)*4;
  if tex[t+3]<128{continue;}depth[idx]=iz;
  for j in 0..3{out[idx*4+j]=(tex[t+j]as f32*shade).clamp(0.,255.)as u8;}out[idx*4+3]=255;
 }}
}
#[cfg(test)]mod tests{
 use super::*;const TEX:&[u8]=include_bytes!("../assets/player-demo/fixture.rgba");
 #[test]fn geometry_changes_with_camera(){let a=render(TEX,false,0.,240,360);let b=render(TEX,false,1.,240,360);assert_ne!(a,b);assert!(a.chunks_exact(4).filter(|p|p[3]>0).count()>1000);}
 #[test]fn slim_changes_real_silhouette(){assert_ne!(render(TEX,false,0.2,240,360),render(TEX,true,0.2,240,360));}
 #[test]fn spring_stays_bounded(){for i in 0..10000{let t=i as f32/120.;assert!((idle_yaw(t)-0.4).abs()<=0.0351);assert!(spring_wave(t,4.).abs()<=1.001);}}
 #[test]fn idle_changes_pose(){assert_ne!(render_idle(TEX,false,0.4,0.,240,360),render_idle(TEX,false,0.4,1.,240,360));}
 #[test]fn transparent_background(){let a=render(TEX,false,0.4,240,360);assert_eq!(&a[0..4],&[0,0,0,0]);assert!(a.chunks_exact(4).all(|p|p[3]==0||p[3]==255));}
}
