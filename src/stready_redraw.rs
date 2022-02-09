use std::{
    sync::{atomic::AtomicU64, Arc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use eframe::epi::Frame;
use log::warn;

pub struct SteadyRedraw(pub Frame, pub Arc<AtomicU64>, pub Instant);

impl SteadyRedraw {
    pub fn redraw_task(self) -> JoinHandle<()> {
        thread::spawn(move || {
            let mut last_redraw_request = Instant::now();
            loop {
                let last_redraw = self.2
                    + Duration::from_millis(self.1.load(std::sync::atomic::Ordering::SeqCst));
                if last_redraw == last_redraw_request {
                    continue;
                }
                last_redraw_request = last_redraw;
                let delta = Instant::now().saturating_duration_since(last_redraw);
                if let Some(dur) = Duration::from_millis(50).checked_sub(delta) {
                    std::thread::sleep(dur);
                } else if cfg!(debug_assertions) {
                    warn!("GUI stalled");
                }
                self.0.request_repaint();
            }
        })
    }
}
