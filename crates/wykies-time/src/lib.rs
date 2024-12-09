//! Simple wrappers to make many errors hard to make

#![warn(unused_crate_dependencies)]

use std::{fmt::Display, time::Duration};

/// Intended to be similar to Duration but always clear that it is in Seconds
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, PartialOrd, Ord,
)]
pub struct Seconds(u64);

/// Intended to be similar to Instant but keeps on ticking if the computer is
/// sleeping, only works with data/time after the unix epoch
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, PartialOrd, Ord,
)]
pub struct Timestamp(u64);

impl Timestamp {
    pub fn now() -> Self {
        Self(
            web_time::SystemTime::UNIX_EPOCH
                .elapsed()
                .expect("expected date on system to be after the epoch")
                .as_secs(),
        )
    }

    pub fn as_local_datetime(&self) -> chrono::DateTime<chrono::Local> {
        chrono::DateTime::from_timestamp(self.0.try_into().unwrap(), 0)
            .expect("wow this program wasn't meant to last that long")
            .into()
    }

    pub fn display_as_locale_datetime(&self) -> String {
        self.as_local_datetime().format("%c").to_string()
    }

    pub fn as_utc_datetime(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(self.0.try_into().unwrap(), 0)
            .expect("wow this program wasn't meant to last that long")
    }

    pub fn display_as_utc_datetime(&self) -> String {
        self.as_utc_datetime().format("%c").to_string()
    }

    pub fn abs_diff(&self, other: Self) -> Seconds {
        self.0.abs_diff(other.0).into()
    }

    pub fn as_secs_since_unix_epoch(&self) -> Seconds {
        self.0.into()
    }

    /// Returns the number of seconds since `past_time` or None if `past_time`
    /// is in the future
    pub fn seconds_since(self, past_time: Self) -> Option<Seconds> {
        if self.0 < past_time.0 {
            None
        } else {
            Some(self - past_time)
        }
    }

    /// Returns the number of seconds since this timestamp or None if this
    /// timestamp is in the future
    pub fn elapsed(self) -> Option<Seconds> {
        Self::now().seconds_since(self)
    }
}

impl std::ops::Add<Seconds> for Timestamp {
    type Output = Self;

    fn add(self, rhs: Seconds) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign<Seconds> for Timestamp {
    fn add_assign(&mut self, rhs: Seconds) {
        self.0 += rhs.0
    }
}

impl std::ops::Sub for Timestamp {
    type Output = Seconds;

    fn sub(self, rhs: Self) -> Self::Output {
        Seconds::new(self.0 - rhs.0)
    }
}

impl From<u32> for Timestamp {
    fn from(value: u32) -> Self {
        Self(value as u64)
    }
}

impl Seconds {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn abs_diff(&self, other: Self) -> Self {
        self.0.abs_diff(other.0).into()
    }

    /// Returns true if this represents zero seconds
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    pub fn saturating_sub(&self, elapsed: Seconds) -> Seconds {
        Self(self.0.saturating_sub(elapsed.0))
    }
}

impl From<u64> for Seconds {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<u8> for Seconds {
    fn from(value: u8) -> Self {
        Self(value.into())
    }
}

impl From<Seconds> for Duration {
    fn from(value: Seconds) -> Self {
        Duration::from_secs(value.0)
    }
}

impl From<Duration> for Seconds {
    fn from(value: Duration) -> Self {
        value.as_secs().into()
    }
}

impl std::ops::Sub for Seconds {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::Mul for Seconds {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl std::ops::Add for Seconds {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

impl From<Seconds> for usize {
    fn from(value: Seconds) -> Self {
        value.0 as _
    }
}

impl Display for Seconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
#[cfg(feature = "mysql")]
pub mod sql {
    use super::*;
    impl sqlx::Encode<'_, sqlx::MySql> for Seconds {
        fn encode_by_ref(
            &self,
            buf: &mut <sqlx::MySql as sqlx::Database>::ArgumentBuffer<'_>,
        ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
            self.0.encode_by_ref(buf)
        }
    }

    impl sqlx::Type<sqlx::MySql> for Seconds {
        fn type_info() -> <sqlx::MySql as sqlx::Database>::TypeInfo {
            u64::type_info()
        }
    }
}
