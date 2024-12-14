use wykies_shared::const_config::client::{
    CLIENT_IDLE_TIMEOUT, CLIENT_TICKS_PER_SECOND_FOR_ACTIVE,
};
use wykies_time::{Seconds, Timestamp};

#[derive(Debug, Default)]
pub struct ScreenLockInfo {
    last_user_activity: Timestamp,
    /// The last time we compared ticks to check for activity
    last_tick_marker: Timestamp,
    is_locked: bool,
    tick_count: usize,
}

impl ScreenLockInfo {
    pub fn is_locked(&mut self) -> bool {
        if self
            .last_user_activity
            .elapsed()
            .expect("user activity should never be in the future")
            > CLIENT_IDLE_TIMEOUT
        {
            self.is_locked = true;
        }
        self.is_locked
    }

    /// Note: if no activity is detected this may immediately relock
    pub fn unlock(&mut self) {
        self.is_locked = false;
    }

    pub fn lock(&mut self) {
        self.is_locked = true;
    }

    pub fn tick(&mut self) {
        self.tick_count += 1;
        let elapsed = self
            .last_tick_marker
            .elapsed()
            .expect("last marker should never be in the future");
        if elapsed >= Seconds::new(1) {
            let has_activity =
                self.tick_count >= CLIENT_TICKS_PER_SECOND_FOR_ACTIVE * usize::from(elapsed);
            if has_activity {
                self.last_user_activity = Timestamp::now();
            }
            self.tick_count = 0;
            self.last_tick_marker = Timestamp::now();
        }
    }
}
