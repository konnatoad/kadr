use std::time::{Duration, Instant};

use crate::config::SlideshowConfig;

pub struct SlideshowEngine {
    pub active: bool,
    last_advance: Instant,
    interval: Duration,
}

impl SlideshowEngine {
    pub fn new(cfg: &SlideshowConfig) -> Self {
        Self {
            active: false,
            last_advance: Instant::now(),
            interval: Duration::from_secs_f64(cfg.interval_secs),
        }
    }

    pub fn update_interval(&mut self, secs: f64) {
        self.interval = Duration::from_secs_f64(secs);
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
        self.last_advance = Instant::now();
    }

    pub fn start(&mut self) {
        self.active = true;
        self.last_advance = Instant::now();
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Returns true if it's time to advance to the next image.
    pub fn tick(&mut self) -> bool {
        if !self.active {
            return false;
        }
        if self.last_advance.elapsed() >= self.interval {
            self.last_advance = Instant::now();
            true
        } else {
            false
        }
    }

    pub fn progress(&self) -> f32 {
        if !self.active { return 0.0; }
        (self.last_advance.elapsed().as_secs_f32() / self.interval.as_secs_f32()).min(1.0)
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.last_advance.elapsed().as_secs_f64()
    }

    pub fn interval_secs(&self) -> f64 {
        self.interval.as_secs_f64()
    }
}
