//! Motion-independent startup gate. No I/O, timers, credentials or window effects.
//! Every asynchronous completion carries an attempt token, so retries/close
//! cannot be undone by a late callback from an earlier worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage { Loading, Intro, AwaitingMainFrame, Handoff, Ready, Failed, Closed }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attempt(u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event { DataLoaded, MainPrepared, IntroFinished, MainFirstFrame, HandoffFinished, Fail }
#[derive(Debug)]
pub struct Gate {
    stage: Stage,
    attempt: Attempt,
    data_loaded: bool,
    main_prepared: bool,
    reduce_motion: bool,
}
impl Gate {
    pub fn new(reduce_motion: bool) -> Self {
        Self {stage:Stage::Loading,attempt:Attempt(0),data_loaded:false,main_prepared:false,reduce_motion}
    }
    pub fn stage(&self) -> Stage {self.stage}
    pub fn attempt(&self) -> Attempt {self.attempt}
    /// Settings can decide this before DataLoaded, never halfway through intro.
    pub fn set_reduce_motion(&mut self, value: bool) {
        if self.stage == Stage::Loading {self.reduce_motion=value;}
    }
    /// Returns true only when this event changed the externally visible stage.
    pub fn accept(&mut self, attempt: Attempt, event: Event) -> bool {
        if attempt != self.attempt || matches!(self.stage,Stage::Failed|Stage::Closed|Stage::Ready) {return false;}
        let previous=self.stage;
        match (self.stage,event) {
            (_,Event::Fail) => self.stage=Stage::Failed,
            (Stage::Loading,Event::DataLoaded) => self.data_loaded=true,
            (Stage::Loading,Event::MainPrepared) => self.main_prepared=true,
            (Stage::Intro,Event::IntroFinished) => self.stage=Stage::AwaitingMainFrame,
            (Stage::AwaitingMainFrame,Event::MainFirstFrame) => self.stage=if self.reduce_motion {Stage::Ready} else {Stage::Handoff},
            (Stage::Handoff,Event::HandoffFinished) => self.stage=Stage::Ready,
            _ => {},
        }
        if self.stage==Stage::Loading && self.data_loaded && self.main_prepared {
            self.stage=if self.reduce_motion {Stage::AwaitingMainFrame} else {Stage::Intro};
        }
        previous != self.stage
    }
    /// Only an explicit retry from failure starts a fresh worker attempt.
    pub fn retry(&mut self) -> Option<Attempt> {
        if self.stage != Stage::Failed {return None;}
        self.attempt=Attempt(self.attempt.0.checked_add(1)?);
        self.data_loaded=false;self.main_prepared=false;self.stage=Stage::Loading;
        Some(self.attempt)
    }
    pub fn close(&mut self) {self.stage=Stage::Closed;}
}
#[cfg(test)]
mod tests {
    use super::*;
    fn prepared(reduce:bool)->Gate {let mut g=Gate::new(reduce);let a=g.attempt();g.accept(a,Event::DataLoaded);g.accept(a,Event::MainPrepared);g}
    #[test] fn requires_both_readiness_signals_in_either_order(){for first in [Event::DataLoaded,Event::MainPrepared] {let mut g=Gate::new(false);let a=g.attempt();assert!(!g.accept(a,first));assert_eq!(g.stage(),Stage::Loading);let second=if first==Event::DataLoaded {Event::MainPrepared} else {Event::DataLoaded};assert!(g.accept(a,second));assert_eq!(g.stage(),Stage::Intro);}}
    #[test] fn early_timer_or_render_signal_cannot_finish(){let mut g=Gate::new(false);let a=g.attempt();for e in [Event::IntroFinished,Event::MainFirstFrame,Event::HandoffFinished] {assert!(!g.accept(a,e));}assert_eq!(g.stage(),Stage::Loading);}
    #[test] fn normal_flow_waits_for_actual_main_frame(){let mut g=prepared(false);let a=g.attempt();g.accept(a,Event::IntroFinished);assert_eq!(g.stage(),Stage::AwaitingMainFrame);g.accept(a,Event::HandoffFinished);assert_eq!(g.stage(),Stage::AwaitingMainFrame);g.accept(a,Event::MainFirstFrame);assert_eq!(g.stage(),Stage::Handoff);g.accept(a,Event::HandoffFinished);assert_eq!(g.stage(),Stage::Ready);}
    #[test] fn reduced_motion_still_waits_for_main_frame(){let mut g=prepared(true);assert_eq!(g.stage(),Stage::AwaitingMainFrame);g.accept(g.attempt(),Event::MainFirstFrame);assert_eq!(g.stage(),Stage::Ready);}
    #[test] fn stale_completion_after_retry_is_ignored(){let mut g=Gate::new(false);let old=g.attempt();g.accept(old,Event::Fail);let new=g.retry().unwrap();assert_ne!(old,new);g.accept(old,Event::DataLoaded);g.accept(new,Event::MainPrepared);assert_eq!(g.stage(),Stage::Loading);g.accept(new,Event::DataLoaded);assert_eq!(g.stage(),Stage::Intro);}
    #[test] fn failure_does_not_resume_itself(){let mut g=prepared(false);let a=g.attempt();g.accept(a,Event::Fail);for e in [Event::DataLoaded,Event::MainPrepared,Event::IntroFinished,Event::MainFirstFrame,Event::HandoffFinished] {assert!(!g.accept(a,e));}assert_eq!(g.stage(),Stage::Failed);}
    #[test] fn close_is_terminal(){let mut g=prepared(false);let a=g.attempt();g.close();g.accept(a,Event::Fail);g.accept(a,Event::IntroFinished);assert_eq!(g.stage(),Stage::Closed);assert_eq!(g.retry(),None);}
    #[test] fn ready_is_terminal(){let mut g=prepared(true);let a=g.attempt();g.accept(a,Event::MainFirstFrame);g.accept(a,Event::Fail);assert_eq!(g.stage(),Stage::Ready);assert_eq!(g.retry(),None);}
    #[test] fn motion_setting_changes_only_while_loading(){let mut g=Gate::new(false);g.set_reduce_motion(true);let a=g.attempt();g.accept(a,Event::DataLoaded);g.accept(a,Event::MainPrepared);g.set_reduce_motion(false);g.accept(a,Event::MainFirstFrame);assert_eq!(g.stage(),Stage::Ready);}
}
