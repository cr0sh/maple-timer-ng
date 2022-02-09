use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{atomic::AtomicBool, Arc},
    thread,
    time::{Duration, Instant},
};

use crossbeam_channel::{Receiver, Sender};
use image::{Bgra, ImageBuffer};
use image_match::Matcher;
use log::trace;
use parking_lot::RwLock;

use crate::rw_condvar::RwCondvar;

pub struct MatchAgent<T: Matcher<ImageBuffer<Bgra<u8>, Vec<u8>>>> {
    last_result: Option<<T as Matcher<ImageBuffer<Bgra<u8>, Vec<u8>>>>::MatchResult>,
    last_recv: Option<Instant>,
    recv: Receiver<(
        <T as Matcher<ImageBuffer<Bgra<u8>, Vec<u8>>>>::MatchResult,
        Instant,
    )>,
    panicked: Arc<AtomicBool>,
    kill: Arc<AtomicBool>,
    suspend: Arc<AtomicBool>,
}

impl<T> MatchAgent<T>
where
    <T as Matcher<ImageBuffer<Bgra<u8>, Vec<u8>>>>::MatchResult: Send + Clone + 'static,
    T: Matcher<ImageBuffer<Bgra<u8>, Vec<u8>>> + Send + 'static,
{
    pub fn new(
        matcher: T,
        cond: Arc<RwCondvar>,
        input_image: Arc<RwLock<Option<ImageBuffer<Bgra<u8>, Vec<u8>>>>>,
        rate_limit: Option<Duration>,
        suspendable: bool,
    ) -> Self {
        let (tx, last_result) = crossbeam_channel::bounded(1);

        let panicked = Arc::new(AtomicBool::new(false));
        let kill = Arc::new(AtomicBool::new(false));
        let suspend = Arc::new(AtomicBool::new(false));
        thread::spawn({
            let cond = Arc::clone(&cond);
            let input_image = Arc::clone(&input_image);
            let panicked = Arc::clone(&panicked);
            let kill = Arc::clone(&kill);
            let suspend = Arc::clone(&suspend);
            move || {
                Self::worker_entrypoint(
                    matcher,
                    cond,
                    input_image,
                    tx,
                    panicked,
                    kill,
                    suspend,
                    suspendable,
                    rate_limit,
                )
            }
        });

        Self {
            last_result: None,
            last_recv: None,
            recv: last_result,
            panicked,
            kill,
            suspend,
        }
    }

    pub fn read_result(
        &mut self,
    ) -> Option<<T as Matcher<ImageBuffer<Bgra<u8>, Vec<u8>>>>::MatchResult> {
        self.recv
            .try_recv()
            .ok()
            .map(|(x, instant)| {
                trace!("Received");
                self.last_result = Some(x.clone());
                self.last_recv = Some(instant);
                x
            })
            .or_else(|| self.last_result.clone())
    }

    pub fn is_panicked(&self) -> bool {
        self.panicked.load(std::sync::atomic::Ordering::SeqCst)
    }

    #[allow(clippy::too_many_arguments)]
    fn worker_entrypoint(
        matcher: T,
        cond: Arc<RwCondvar>,
        input_image: Arc<RwLock<Option<ImageBuffer<Bgra<u8>, Vec<u8>>>>>,
        result_tx: Sender<(
            <T as Matcher<ImageBuffer<Bgra<u8>, Vec<u8>>>>::MatchResult,
            Instant,
        )>,
        panicked: Arc<AtomicBool>,
        kill: Arc<AtomicBool>,
        suspend: Arc<AtomicBool>,
        suspendable: bool,
        rate_limit: Option<Duration>,
    ) {
        trace!("worker_entrypoint");
        let rate_limit = rate_limit.unwrap_or(Duration::ZERO);
        let result = catch_unwind(AssertUnwindSafe(move || {
            let mut buffer = Vec::new();
            let mut last_match = Instant::now() - rate_limit;
            loop {
                if kill.load(std::sync::atomic::Ordering::SeqCst) {
                    return;
                }
                {
                    let mut guard = input_image.read();
                    cond.wait_read(&mut guard);
                    if last_match + rate_limit > Instant::now() {
                        continue;
                    }
                    last_match = Instant::now();
                    let inner = if let Some(inner) = &*guard {
                        inner
                    } else {
                        continue;
                    };
                    if suspendable && suspend.load(std::sync::atomic::Ordering::SeqCst) {
                        continue;
                    }

                    buffer.truncate(0);
                    buffer.extend_from_slice(inner.as_raw());
                    let img = ImageBuffer::<Bgra<u8>, Vec<u8>>::from_raw(
                        inner.width(),
                        inner.height(),
                        buffer,
                    )
                    .unwrap();

                    drop(guard);

                    for candidate in matcher.candidates_iter(&img) {
                        if matcher.check(&candidate) {
                            if let Some(result) = matcher.match_image(&candidate) {
                                // FIXME: This does not overwrite last result if the recevier stalls
                                if result_tx.try_send((result, last_match)).is_ok() {
                                    trace!("Found match result");
                                    if suspendable {
                                        trace!("Suspending");
                                        suspend.store(true, std::sync::atomic::Ordering::SeqCst);
                                    }
                                };
                                break;
                            }
                        }
                    }

                    buffer = img.into_raw();
                }
            }
        }));

        if result.is_err() {
            panicked.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    /// Get the match agent's last recv.
    pub fn last_recv(&self) -> Option<Instant> {
        self.last_recv
    }

    pub fn wake(&self) {
        self.suspend
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

impl<T: image_match::Matcher<image::ImageBuffer<image::Bgra<u8>, std::vec::Vec<u8>>>> Drop
    for MatchAgent<T>
{
    fn drop(&mut self) {
        self.kill.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}
