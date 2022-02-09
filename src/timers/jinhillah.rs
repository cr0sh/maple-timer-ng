use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use image::{Bgra, ImageBuffer};
use image_match::{
    jinhillah::{JinHillahHpMatcher, JinHillahReapMatcher},
    BoundsCachedMatcher,
};
use log::trace;
use parking_lot::RwLock;

use crate::{rw_condvar::RwCondvar, MatchAgent};

use super::Timer;

pub struct JinhillahTimer {
    hp: MatchAgent<BoundsCachedMatcher<JinHillahHpMatcher>>,
    reap: MatchAgent<BoundsCachedMatcher<JinHillahReapMatcher>>,
    normal_mode: bool,
    capture_time: Option<Instant>,
    duration_at_capture: Duration,
}

impl JinhillahTimer {
    pub fn new(
        cond: Arc<RwCondvar>,
        image_lock: Arc<RwLock<Option<ImageBuffer<Bgra<u8>, Vec<u8>>>>>,
        dimensions: (u32, u32),
        normal_mode: bool,
    ) -> Self {
        Self {
            hp: MatchAgent::new(
                BoundsCachedMatcher::new(JinHillahHpMatcher),
                Arc::clone(&cond),
                Arc::clone(&image_lock),
                None,
                false,
            ),
            reap: MatchAgent::new(
                BoundsCachedMatcher::new(JinHillahReapMatcher(dimensions.0, dimensions.1)),
                cond,
                image_lock,
                Some(Duration::from_millis(490)),
                true,
            ),
            normal_mode,
            capture_time: None,
            duration_at_capture: if normal_mode {
                Duration::from_secs(180)
            } else {
                Duration::from_secs(150)
            },
        }
    }
}

impl JinhillahTimer {
    fn duration_realtime(&mut self) -> Duration {
        const HARD_DURATIONS: [Duration; 3] = [
            Duration::from_secs(150),
            Duration::from_secs(125),
            Duration::from_secs(100),
        ];
        const NORMAL_DURATIONS: [Duration; 3] = [
            Duration::from_secs(180),
            Duration::from_secs(155),
            Duration::from_secs(120),
        ];

        let total_hp_ratio = self.total_hp_ratio();

        let durations = if self.normal_mode {
            NORMAL_DURATIONS
        } else {
            HARD_DURATIONS
        };

        if total_hp_ratio < 0.3 {
            durations[2]
        } else if total_hp_ratio < 0.6 {
            durations[1]
        } else {
            durations[0]
        }
    }

    fn total_hp_ratio(&mut self) -> f64 {
        self.hp
            .read_result()
            .map(|x| (4 - x.phase()) as f64 * 0.25 + x.hp_ratio() * 0.25)
            .unwrap_or(1.0)
    }
}

impl Timer for JinhillahTimer {
    fn duration(&mut self) -> Duration {
        self.duration_at_capture
    }

    fn last_match(&mut self) -> Option<Instant> {
        let ret = self.reap.read_result().and_then(|_| self.reap.last_recv());
        match (ret, &self.capture_time) {
            (Some(x), None) => {
                self.capture_time = Some(x);
                self.duration_at_capture = self.duration_realtime();
            }
            (Some(x), Some(y)) if x >= *y + self.duration_at_capture => {
                self.capture_time = Some(x);
                self.duration_at_capture = self.duration_realtime();
            }
            _ => (),
        };

        self.capture_time
    }

    fn text(&self) -> &str {
        if self.normal_mode {
            "진 힐라 (노말)"
        } else {
            "진 힐라 (하드)"
        }
    }

    fn red_threshold(&self) -> Duration {
        Duration::from_secs(10)
    }

    fn yellow_threshold(&self) -> Duration {
        Duration::from_secs(30)
    }

    fn is_panicked(&self) -> bool {
        self.hp.is_panicked() || self.reap.is_panicked()
    }

    fn wake(&mut self) {
        trace!("JinhillahTimer reap wakeup");
        self.reap.wake();
    }

    fn debug_string(&mut self) -> String {
        let result = self.hp.read_result();
        let raw = result
            .as_ref()
            .map(|x| format!("{:?}", x))
            .unwrap_or_else(|| String::from("?"));
        let phase = result
            .as_ref()
            .map(|x| x.phase().to_string())
            .unwrap_or_else(|| String::from("?"));
        let ratio = result
            .map(|x| format!("{:.3}", x.hp_ratio()))
            .unwrap_or_else(|| String::from("?"));
        format!(
            "dur: {:.2}, raw: {raw}, phase: {phase} ratio: {ratio}, totalRatio: {:.4}",
            self.duration().as_secs_f64(),
            self.total_hp_ratio(),
        )
    }
}
