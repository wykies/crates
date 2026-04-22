use wykies_time::{Seconds, Timestamp};

#[derive(Debug)]
pub struct ScreenLockInfo {
    last_user_activity: Timestamp,
    /// The last time we compared ticks to check for activity
    last_tick_marker: Timestamp,
    is_locked: bool,
    tick_count: usize,
    client_idle_timeout: Seconds,
    client_ticks_per_second_for_active: usize,
}

impl ScreenLockInfo {
    pub fn new(client_idle_timeout: Seconds, client_ticks_per_second_for_active: usize) -> Self {
        Self {
            last_user_activity: Timestamp::now(),
            last_tick_marker: Timestamp::now(),
            is_locked: false,
            tick_count: 0,
            client_idle_timeout,
            client_ticks_per_second_for_active,
        }
    }

    pub fn is_locked(&mut self) -> bool {
        if self
            .last_user_activity
            .elapsed()
            .expect("user activity should never be in the future")
            > self.client_idle_timeout
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

    /// Call this exactly once in your UI update loop
    /// It will update the tick count and use that to determine if there is user
    /// activity based on the threshold that was passed in when it was setup
    pub fn tick(&mut self) {
        self.tick_count += 1;
        let elapsed = self
            .last_tick_marker
            .elapsed()
            .expect("last marker should never be in the future");
        if elapsed >= Seconds::new(1) {
            let has_activity =
                self.tick_count >= self.client_ticks_per_second_for_active * usize::from(elapsed);
            if has_activity {
                self.last_user_activity = Timestamp::now();
            }
            self.tick_count = 0;
            self.last_tick_marker = Timestamp::now();
        }
    }
}
