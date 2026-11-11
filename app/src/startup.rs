//! Pure startup sequence. No network, account, window or filesystem side effects.
#[derive(Clone, Copy, Debug)]
pub struct Glyph { pub visible: bool, pub x: f64, pub y: f64, pub sx: f64, pub sy: f64 }
#[derive(Clone, Debug)]
pub struct Frame { pub x: f64, pub y: f64, pub size: f64, pub glyphs: [Glyph;9] }
fn ease(x: f64)->f64 { let t=x.clamp(0.,1.); t*t*t*(t*(t*6.-15.)+10.) }
pub const INTRO_SECONDS: f64 = 2.1;
pub fn frame(seconds: f64)->Frame {
    let f=seconds.max(0.)*60.; let q=ease(f/94.);
    let xs=[566.5425816900001,672.9801216900001,781.0859616900002,884.0756816900002,977.7229216900002,1085.0502216900002,1222.5181416900002,1310.82682169,1408.5892016900002];
    let shift=-1120.*(1.-q);
    let glyphs=std::array::from_fn(|i| Glyph { visible:true,x:xs[i]+shift,y:550.4080013399999,sx:1.,sy:1. });
    Frame {x:960.+(480.96902124999997-960.)*q,y:540.+(508.59999932999995-540.)*q,size:650.+(160.76932338-650.)*q,glyphs}
}
#[cfg(test)] mod tests {
 use super::*;
 #[test] fn planet_moves_continuously_left_and_shrinks() { let mut prev=frame(0.); for f in 1..126 {let next=frame(f as f64/60.);assert!(next.x<=prev.x+1e-8);assert!(next.size<=prev.size+1e-8);prev=next;} }
 #[test] fn whole_train_has_fixed_spacing_and_sizes() {let end=frame(2.1);for f in 0..126 {let st=frame(f as f64/60.);let shift=st.glyphs[0].x-end.glyphs[0].x;for (g,e) in st.glyphs.iter().zip(end.glyphs.iter()){assert!((g.x-e.x-shift).abs()<1e-9);assert_eq!((g.sx,g.sy),(1.,1.));assert!(g.visible);}}}
}

/// Approved dark-arc fallback as a filled round-cap ribbon, avoiding
/// backend-dependent Path stroke caps. Coordinates are planet-local units.
pub fn loader_arc(seconds: f64) -> String {
    let tau=std::f64::consts::TAU;
    let center=seconds.rem_euclid(4./3.)/(4./3.)*tau;
    let span=70f64.to_radians(); let step=0.25f64.to_radians();
    let psi=(-14.75f64).to_radians(); let mut out=String::new();
    let mut run:Vec<(f64,f64)>=Vec::new();let mut previous_near=false;
    fn ribbon(out:&mut String,run:&[(f64,f64)]) {
        use std::fmt::Write;
        if run.len()<2 {return;}
        let mut left=Vec::new();let mut right=Vec::new();
        for i in 0..run.len(){
            let a=run[if i==0 {0} else {i-1}];let b=run[(i+1).min(run.len()-1)];
            let dx=b.0-a.0;let dy=b.1-a.1;let len=dx.hypot(dy);
            let nx=-dy/len*2.3;let ny=dx/len*2.3;
            left.push((run[i].0+nx,run[i].1+ny));right.push((run[i].0-nx,run[i].1-ny));
        }
        let _=write!(out,"M {:.4} {:.4} ",left[0].0,left[0].1);
        for p in left.iter().skip(1){let _=write!(out,"L {:.4} {:.4} ",p.0,p.1);}
        let end=right[right.len()-1];let _=write!(out,"A 2.3 2.3 0 0 0 {:.4} {:.4} ",end.0,end.1);
        for p in right.iter().rev().skip(1){let _=write!(out,"L {:.4} {:.4} ",p.0,p.1);}
        let start=left[0];let _=write!(out,"A 2.3 2.3 0 0 0 {:.4} {:.4} Z ",start.0,start.1);
    }
    for i in 0..=280 {
        let u=center-span/2.+i as f64*step;let a=36.*u.cos();let b=9.1101547*u.sin();
        let dx=a*psi.cos()-b*psi.sin();let dy=a*psi.sin()+b*psi.cos();
        let near=u.rem_euclid(tau)<std::f64::consts::PI;
        let visible=near || dx*dx+dy*dy>=26.535f64.powi(2);
        if !visible || (!run.is_empty() && near!=previous_near) {ribbon(&mut out,&run);run.clear();}
        if visible {run.push((49.485+dx,50.533+dy));}
        previous_near=near;
    }
    ribbon(&mut out,&run);out
}
