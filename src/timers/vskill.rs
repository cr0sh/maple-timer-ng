use std::{sync::Arc, time::Duration};

use assets_manager::asset::Png;
use image::{Bgra, ImageBuffer};
use image_match::buff::BuffMatcher;
use parking_lot::RwLock;

use crate::rw_condvar::RwCondvar;

use super::{match_agent::MatchAgent, Timer};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum VSkillKind {
    FatalStrike,
}

pub struct VSkillTimer {
    matcher: MatchAgent<BuffMatcher>,
    kind: VSkillKind,
}

impl VSkillTimer {
    pub fn new(
        cond: Arc<RwCondvar>,
        image_lock: Arc<RwLock<Option<ImageBuffer<Bgra<u8>, Vec<u8>>>>>,
        kind: VSkillKind,
        dims: (u32, u32),
    ) -> Self {
        Self {
            matcher: MatchAgent::new(
                BuffMatcher::new(
                    assets_embedded::assets()
                        .load::<Png>("v_buficon")
                        .unwrap()
                        .cloned()
                        .0
                        .to_bgra8(),
                    0.8,
                    dims,
                ),
                Arc::clone(&cond),
                Arc::clone(&image_lock),
                None,
                true,
            ),
            kind,
        }
    }
}

impl Timer for VSkillTimer {
    fn duration(&mut self) -> Duration {
        match self.kind {
            VSkillKind::FatalStrike => Duration::from_secs(30),
        }
    }

    fn last_match(&mut self) -> Option<std::time::Instant> {
        self.matcher
            .read_result()
            .and_then(|_| self.matcher.last_recv())
    }

    fn remaining_time(&mut self) -> Option<Duration> {
        self.last_match()
            .map(|x| x + self.duration())
            .map(|x| x.saturating_duration_since(std::time::Instant::now()))
    }

    fn text(&self) -> &str {
        match self.kind {
            VSkillKind::FatalStrike => "일격필살 I",
        }
    }

    fn yellow_threshold(&self) -> Duration {
        Duration::from_secs(10)
    }

    fn red_threshold(&self) -> Duration {
        Duration::from_secs(3)
    }

    fn is_panicked(&self) -> bool {
        self.matcher.is_panicked()
    }

    fn debug_string(&mut self) -> String {
        String::new()
    }

    fn wake(&mut self) {
        self.matcher.wake()
    }
}
