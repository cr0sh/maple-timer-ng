use parking_lot::{Condvar, Mutex, RwLockReadGuard};

// https://github.com/Amanieu/parking_lot/issues/165
pub struct RwCondvar {
    c: Condvar,
    m: Mutex<()>,
}

impl RwCondvar {
    pub fn new() -> Self {
        Self {
            c: Condvar::new(),
            m: Mutex::new(()),
        }
    }

    pub fn cond(&self) -> &Condvar {
        &self.c
    }

    pub fn wait_read<T>(&self, g: &mut RwLockReadGuard<'_, T>) {
        let guard = self.m.lock();
        RwLockReadGuard::unlocked(g, || {
            // Move the guard in so it gets unlocked before we re-lock g
            let mut guard = guard;
            self.c.wait(&mut guard);
        });
    }
}
