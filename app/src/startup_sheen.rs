//! One-shot light pass after the accepted intro. Geometry and earlier timing stay unchanged.
pub const SECONDS:f64=0.9;
pub const SETTLE:f64=0.12;
pub const END:f64=crate::startup::INTRO_SECONDS+SECONDS+SETTLE;
pub fn position(seconds:f64,reduced:bool)->f32{
 let t=(seconds-crate::startup::INTRO_SECONDS)/SECONDS;
 if reduced||seconds<=crate::startup::INTRO_SECONDS||seconds>=crate::startup::INTRO_SECONDS+SECONDS{return -1.;}
 let q=t*t*t*(10.+t*(-15.+6.*t));
 (200.+1550.*q)as f32
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn only_after_complete_logo(){for i in 0..=210{assert_eq!(position(i as f64/100.,false),-1.);}assert_eq!(position(3.,false),-1.);assert_eq!(position(10.,false),-1.);}
 #[test]fn one_way_one_pass(){let mut prev=0.;for i in 1..900{let p=position(2.1+i as f64/1000.,false);assert!(p>=prev);assert!((200. ..=1750.).contains(&p));prev=p;}}
 #[test]fn reduced_has_no_pass(){for i in 0..=500{assert_eq!(position(i as f64/100.,true),-1.);}}
}
