use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
};

use anyhow::Context;
use chrono::{DateTime, Local, NaiveDateTime, NaiveTime};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AppState {
    last_alive_msg: DateTime<Local>,
    alive_msg_time: Option<NaiveTime>,
    logs_dir: PathBuf,
    latest_log_datetime: NaiveDateTime,
    allowed_num_hours_without_log: Option<i64>,
    #[serde(skip)]
    is_changed: bool,
}
impl AppState {
    pub fn is_changed(&self) -> bool {
        self.is_changed
    }

    pub(crate) fn save<P: AsRef<Path>>(&mut self, path: P) -> anyhow::Result<()> {
        let s = ron::ser::to_string_pretty(&self, ron::ser::PrettyConfig::default())
            .context("failed to convert to ron")?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .with_context(|| {
                format!("failed to open file to save AppState: {:?}", path.as_ref())
            })?;
        file.write_all(s.as_bytes()).with_context(|| {
            format!(
                "failed to write to file to save AppState: {:?}",
                path.as_ref()
            )
        })?;
        self.is_changed = false;
        Ok(())
    }

    pub(crate) fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let s = fs::read_to_string(&path)
            .with_context(|| format!("failed to read file for AppState: {:?}", path.as_ref()))?;
        ron::from_str(&s).with_context(|| {
            format!(
                "failed to deserialize AppState from contents of {:?}",
                path.as_ref(),
            )
        })
    }

    pub fn new(logs_dir: PathBuf) -> Self {
        Self {
            last_alive_msg: Local::now(),
            alive_msg_time: Some(
                NaiveTime::from_hms_opt(7, 0, 0)
                    .expect("should be valid as it is set at build time"),
            ),
            allowed_num_hours_without_log: Some(24),
            latest_log_datetime: Local::now().naive_local(),
            logs_dir,
            is_changed: Default::default(),
        }
    }

    pub fn new_with_min_dates(logs_dir: PathBuf) -> Self {
        let mut result = Self::new(logs_dir);
        result.last_alive_msg = NaiveDateTime::MIN.and_local_timezone(Local).unwrap();
        result.latest_log_datetime = NaiveDateTime::MIN;
        result
    }

    pub(crate) fn generate_alive_msg(&mut self) -> String {
        self.last_alive_msg = Local::now();
        self.is_changed = true;
        format!(
            "FS Log Monitor still working. Version: {}",
            env!("CARGO_PKG_VERSION")
        )
    }

    /// Due if message not sent for the day and past the time to send the message
    pub(crate) fn alive_msg_due(&self) -> bool {
        if let Some(send_time) = self.alive_msg_time {
            let now = Local::now();
            if self.last_alive_msg.date_naive() != now.date_naive() {
                now.time() >= send_time
            } else {
                // Same date as last message no due yet
                false
            }
        } else {
            // Always false not set to be sent
            false
        }
    }

    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    pub fn latest_log_datetime(&self) -> NaiveDateTime {
        self.latest_log_datetime
    }

    pub fn set_latest_log_datetime(&mut self, value: NaiveDateTime) {
        self.is_changed = true;
        self.latest_log_datetime = value;
    }

    pub(crate) fn generate_inactivity_msg(&self) -> Option<String> {
        let allowed_hours = self.allowed_num_hours_without_log?;
        let num_hours_since_log = Local::now()
            .naive_local()
            .signed_duration_since(self.latest_log_datetime)
            .num_hours();
        if num_hours_since_log > allowed_hours {
            Some(format!("Most recent log found ({}) exceeds the allowed number of hours ({allowed_hours}) without a log. Currently {num_hours_since_log} hours without a log.", self.latest_log_datetime.format("%F %T")))
        } else {
            None
        }
    }
}
