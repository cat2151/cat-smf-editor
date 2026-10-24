//! Bounded handoff for short main-thread metadata transactions.
use std::sync::{Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};

pub fn render_lock<T>(mutex: &Mutex<T>, frames: usize, rate: u32) -> Option<MutexGuard<'_, T>> {
    match mutex.try_lock() {
        Ok(guard) => return Some(guard),
        Err(TryLockError::Poisoned(_)) => return None,
        Err(TryLockError::WouldBlock) => {}
    }
    // Do not discard a whole device buffer for a brief metadata handoff.
    // Bound spinning to a quarter of this buffer's time, at most 2ms, leaving
    // the rest of the callback deadline for actual rendering. Never block on
    // the UI mutex or wait for a slow plugin without a deadline.
    let budget = Duration::from_secs_f64(frames as f64 / f64::from(rate) / 4.0)
        .min(Duration::from_millis(2));
    let start = Instant::now();
    while start.elapsed() < budget {
        match mutex.try_lock() {
            Ok(guard) => return Some(guard),
            Err(TryLockError::Poisoned(_)) => return None,
            Err(TryLockError::WouldBlock) => std::hint::spin_loop(),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_lock_needs_no_wait_budget() {
        assert_eq!(*render_lock(&Mutex::new(7), 0, 48000).unwrap(), 7);
    }

    #[test]
    fn busy_lock_with_no_budget_returns_instead_of_blocking() {
        let mutex = Mutex::new(7);
        let _held = mutex.lock().unwrap();
        assert!(render_lock(&mutex, 0, 48000).is_none());
    }

    #[test]
    fn poisoned_render_state_is_not_used() {
        let mutex = Mutex::new(7);
        let _ = std::panic::catch_unwind(|| {
            let _held = mutex.lock().unwrap();
            panic!("poison fixture");
        });
        assert!(render_lock(&mutex, 480, 48000).is_none());
    }
}
