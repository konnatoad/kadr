use std::time::{Duration, Instant};

use crate::config::SlideshowConfig;

/// What the app should do after each call to [`SlideshowEngine::tick`].
#[derive(Debug, PartialEq)]
pub enum TickResult {
    /// Nothing to do this frame.
    Nothing,
    /// The hold interval ended.
    /// App must: save `current_texture` → `prev_texture`, advance index, load next image.
    BeginTransition,
    /// Crossfade in progress.  `t` runs 0.0 (fully prev) → 1.0 (fully current).
    TransitionProgress(f32),
    /// Crossfade finished.  App must drop `prev_texture`.
    TransitionDone,
}

#[derive(PartialEq)]
enum Phase {
    Holding,
    Transitioning,
}

pub struct SlideshowEngine {
    pub active: bool,
    /// Crossfade duration in seconds.  Set from config; can be overridden at runtime.
    pub transition_secs: f32,
    phase: Phase,
    phase_start: Instant,
    interval: Duration,
}

impl SlideshowEngine {
    pub fn new(cfg: &SlideshowConfig) -> Self {
        Self {
            active: false,
            transition_secs: cfg.transition_secs,
            phase: Phase::Holding,
            phase_start: Instant::now(),
            interval: Duration::from_secs_f64(cfg.interval_secs),
        }
    }

    pub fn update_interval(&mut self, secs: f64) {
        self.interval = Duration::from_secs_f64(secs);
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
        self.phase = Phase::Holding;
        self.phase_start = Instant::now();
    }

    pub fn start(&mut self) {
        self.active = true;
        self.phase = Phase::Holding;
        self.phase_start = Instant::now();
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.phase = Phase::Holding;
    }

    /// Drive the slideshow one frame forward.
    pub fn tick(&mut self) -> TickResult {
        if !self.active {
            return TickResult::Nothing;
        }
        match self.phase {
            Phase::Holding => {
                if self.phase_start.elapsed() >= self.interval {
                    self.phase = Phase::Transitioning;
                    self.phase_start = Instant::now();
                    TickResult::BeginTransition
                } else {
                    TickResult::Nothing
                }
            }
            Phase::Transitioning => {
                let dur = self.transition_secs.max(0.01);
                let t = (self.phase_start.elapsed().as_secs_f32() / dur).min(1.0);
                if t >= 1.0 {
                    self.phase = Phase::Holding;
                    self.phase_start = Instant::now();
                    TickResult::TransitionDone
                } else {
                    TickResult::TransitionProgress(t)
                }
            }
        }
    }

    /// 0 → 1 progress within the current *hold* phase (used by UI progress bars).
    pub fn progress(&self) -> f32 {
        if !self.active {
            return 0.0;
        }
        match self.phase {
            Phase::Holding => {
                (self.phase_start.elapsed().as_secs_f32() / self.interval.as_secs_f32()).min(1.0)
            }
            Phase::Transitioning => 1.0,
        }
    }

    /// Seconds elapsed in the current hold phase (for Lua `ctx.elapsed_secs`).
    pub fn elapsed_secs(&self) -> f64 {
        match self.phase {
            Phase::Holding => self.phase_start.elapsed().as_secs_f64(),
            // Report full interval during transition so Lua sees a stable value.
            Phase::Transitioning => self.interval.as_secs_f64(),
        }
    }

    pub fn interval_secs(&self) -> f64 {
        self.interval.as_secs_f64()
    }

    pub fn in_transition(&self) -> bool {
        self.phase == Phase::Transitioning
    }
}
