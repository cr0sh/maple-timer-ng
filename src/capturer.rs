use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::Context;
use image::{Bgra, ImageBuffer};
use log::trace;
use parking_lot::RwLock;
use winscr::gdi_capture::GdiCapturer;

use crate::rw_condvar::RwCondvar;

pub struct Capturer {
    kill: Arc<AtomicBool>,
    dims: Arc<(AtomicU32, AtomicU32)>,
    cond: Arc<RwCondvar>,
    lock: Arc<RwLock<Option<ImageBuffer<Bgra<u8>, Vec<u8>>>>>,
    panicked: Arc<AtomicBool>,
}

impl Capturer {
    pub fn new(hidpi: bool) -> anyhow::Result<Self> {
        let kill = Arc::new(AtomicBool::new(false));
        let panicked = Arc::new(AtomicBool::new(false));
        let cond = Arc::new(RwCondvar::new());
        let lock = Arc::new(RwLock::new(None));

        let dims = Arc::new((AtomicU32::new(0), AtomicU32::new(0)));
        std::thread::spawn({
            let kill = Arc::clone(&kill);
            let panicked = Arc::clone(&panicked);
            let cond = Arc::clone(&cond);
            let dims = Arc::clone(&dims);
            let lock = Arc::clone(&lock);
            move || {
                Self::capture_task(kill, cond, lock, dims, panicked, hidpi);
            }
        });
        Ok(Self {
            kill,
            cond,
            lock,
            dims,
            panicked,
        })
    }

    fn capture_task(
        kill: Arc<AtomicBool>,
        cond: Arc<RwCondvar>,
        lock: Arc<RwLock<Option<ImageBuffer<Bgra<u8>, Vec<u8>>>>>,
        dims: Arc<(AtomicU32, AtomicU32)>,
        panicked: Arc<AtomicBool>,
        hidpi: bool,
    ) {
        trace!("capture_task");
        let result = catch_unwind(AssertUnwindSafe(move || {
            let mut capturer = GdiCapturer::new("MapleStory", "MapleStoryClass", hidpi)
                .context("Cannot initialize capturer")
                .unwrap();
            dims.0
                .store(capturer.dimension().0, std::sync::atomic::Ordering::SeqCst);
            dims.1
                .store(capturer.dimension().1, std::sync::atomic::Ordering::SeqCst);
            let mut last_cap = Instant::now();
            let capture_duration = Duration::from_millis(50);
            loop {
                std::thread::sleep(
                    (last_cap + capture_duration).saturating_duration_since(Instant::now()),
                );

                if kill.load(std::sync::atomic::Ordering::SeqCst) {
                    trace!("capture_task killed");
                    return;
                }

                let _ = capturer.capture();
                let buf = capturer.get_image_buffer().map(|x| {
                    ImageBuffer::from_raw(x.width(), x.height(), x.as_raw().to_vec()).unwrap()
                });
                last_cap = Instant::now();
                *lock.write() = buf;
                cond.cond().notify_all();
            }
        }));

        if result.is_err() {
            panicked.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    /// Get the capturer's dims.
    pub fn dims(&self) -> (u32, u32) {
        (
            self.dims.0.load(std::sync::atomic::Ordering::SeqCst),
            self.dims.1.load(std::sync::atomic::Ordering::SeqCst),
        )
    }

    /// Get a reference to the capturer's cond.
    pub fn cond(&self) -> &Arc<RwCondvar> {
        &self.cond
    }

    /// Get a reference to the capturer's lock.
    pub fn lock_ref(&self) -> &Arc<RwLock<Option<ImageBuffer<Bgra<u8>, Vec<u8>>>>> {
        &self.lock
    }

    pub fn is_panicked(&self) -> bool {
        self.panicked.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Drop for Capturer {
    fn drop(&mut self) {
        self.kill.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}
