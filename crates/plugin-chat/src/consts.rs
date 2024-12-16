//! Constant here for simplicity but most can be moved to the settings if they
//! need to be configurable

use wykies_time::Seconds;

pub const CHAT_HISTORY_RECENT_CAPACITY: usize = 100;
pub const CHAT_HISTORY_REQUEST_SIZE: u8 = 50;
/// This controls the max number of messages buffered before saving to the
/// DB
///
/// NOTE: Should be less than `CHAT_HISTORY_RECENT_CAPACITY`
/// otherwise some history will not be accessible to recently joined users
pub const CHAT_MAX_IMS_BEFORE_SAVE: u8 = 80;
pub const CHAT_MAX_TIME_BEFORE_SAVE: Seconds = Seconds::new(30);
// TODO 6: Rate limit on server end
pub const CHAT_MIN_TIME_BETWEEN_HISTORY_REQUESTS: Seconds = Seconds::new(5);
pub const CHAT_SYSTEM_USERNAME: &str = "System";

#[cfg(test)]
mod tests {
    use static_assertions::const_assert;

    use super::{CHAT_HISTORY_RECENT_CAPACITY, CHAT_MAX_IMS_BEFORE_SAVE};

    const_assert!((CHAT_MAX_IMS_BEFORE_SAVE as usize) < CHAT_HISTORY_RECENT_CAPACITY);
}
