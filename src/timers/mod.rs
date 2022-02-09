use std::time::{Duration, Instant};

pub mod jinhillah;
pub mod match_agent;

pub trait Timer {
    fn duration(&mut self) -> Duration;
    fn last_match(&mut self) -> Option<Instant>;
    fn remaining_time(&mut self) -> Option<Duration> {
        self.last_match()
            .map(|x| x + self.duration())
            .map(|x| x.saturating_duration_since(Instant::now()))
    }
    fn text(&self) -> &str;
    fn yellow_threshold(&self) -> Duration {
        Duration::from_secs(10)
    }
    fn red_threshold(&self) -> Duration {
        Duration::from_secs(3)
    }
    fn is_panicked(&self) -> bool;
    fn debug_string(&mut self) -> String {
        String::new()
    }
    fn wake(&mut self);
}
