// App lock: hides history behind OS authentication (Touch ID / password on
// macOS, Windows Hello on Windows) after Clipsa has been in the background.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::error::{ClipsaError, ClipsaResult};
use crate::platform::Platform;

pub const LOCKED_MESSAGE: &str = "Clipsa is locked";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LockState {
    pub supported: bool,
    pub locked: bool,
}

pub struct AppLock {
    locked: AtomicBool,
    authenticating: AtomicBool,
    /// When Clipsa stopped being active (no window focused/visible).
    inactive_since: Mutex<Option<Instant>>,
}

impl AppLock {
    pub fn new(locked: bool) -> Self {
        Self {
            locked: AtomicBool::new(locked),
            authenticating: AtomicBool::new(false),
            inactive_since: Mutex::new(None),
        }
    }

    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::SeqCst)
    }

    /// True while an OS authentication prompt is open (it takes focus away
    /// from Clipsa windows, which must not count as "inactive").
    pub fn is_authenticating(&self) -> bool {
        self.authenticating.load(Ordering::SeqCst)
    }

    pub fn ensure_unlocked(&self) -> ClipsaResult<()> {
        if self.is_locked() {
            Err(ClipsaError::Rejected(LOCKED_MESSAGE.into()))
        } else {
            Ok(())
        }
    }

    pub fn lock(&self) {
        self.locked.store(true, Ordering::SeqCst);
        *self
            .inactive_since
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = None;
    }

    /// Run an OS authentication prompt with the `authenticating` flag set.
    pub fn authenticate(&self, platform: &Platform, reason: &str) -> ClipsaResult<()> {
        if !platform.auth_supported() {
            return Err(ClipsaError::Rejected(
                "App lock isn't available on this device".into(),
            ));
        }
        self.authenticating.store(true, Ordering::SeqCst);
        let result = platform.authenticate(reason);
        self.authenticating.store(false, Ordering::SeqCst);
        result
    }

    pub fn unlock(&self, platform: &Platform) -> ClipsaResult<()> {
        if !self.is_locked() {
            return Ok(());
        }
        self.authenticate(platform, "unlock your clipboard history")?;
        *self
            .inactive_since
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = None;
        self.locked.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Periodic check. `active` = Clipsa is in use (a window is focused, or
    /// for `lock_after == 0` merely visible). Returns true when it just locked.
    pub fn tick(&self, enabled: bool, lock_after: Duration, active: bool, now: Instant) -> bool {
        let mut inactive_since = self
            .inactive_since
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if !enabled || self.is_locked() || active || self.is_authenticating() {
            *inactive_since = None;
            return false;
        }
        let since = *inactive_since.get_or_insert(now);
        if now.duration_since(since) >= lock_after {
            drop(inactive_since);
            self.lock();
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locks_only_after_being_inactive_for_the_whole_delay() {
        let lock = AppLock::new(false);
        let start = Instant::now();
        let delay = Duration::from_secs(60);
        assert!(!lock.tick(true, delay, false, start));
        assert!(!lock.tick(true, delay, false, start + Duration::from_secs(59)));
        // Becoming active resets the countdown.
        assert!(!lock.tick(true, delay, true, start + Duration::from_secs(59)));
        assert!(!lock.tick(true, delay, false, start + Duration::from_secs(60)));
        assert!(!lock.is_locked());
        assert!(lock.tick(true, delay, false, start + Duration::from_secs(120)));
        assert!(lock.is_locked());
        assert_eq!(
            lock.ensure_unlocked().unwrap_err().to_string(),
            LOCKED_MESSAGE
        );
    }

    #[test]
    fn never_locks_when_disabled_and_zero_delay_locks_immediately() {
        let lock = AppLock::new(false);
        let now = Instant::now();
        assert!(!lock.tick(false, Duration::ZERO, false, now));
        assert!(lock.tick(true, Duration::ZERO, false, now));
    }
}
