use std::{fs, path::Path, time::Duration};

use anyhow::{bail, Context};
use serde_json::json;

pub struct Discord {
    url: String,
}

impl Discord {
    const RETRY_ATTEMPTS: u8 = 3;
    const INTERVAL_BETWEEN_RETRY: Duration = Duration::from_secs(30);

    pub fn new(config_folder: &Path) -> anyhow::Result<Self> {
        let filename = config_folder.join("d.data");
        let url_suffix = fs::read_to_string(&filename).with_context(|| {
            format!("failed to read discord webhook url suffix from {filename:?}")
        })?;
        let url = format!("https://discord.com/api/webhooks/{url_suffix}");
        Ok(Self { url })
    }

    pub fn send(msg: &str, config_folder: &Path) -> anyhow::Result<()> {
        let discord = Self::new(config_folder)?;

        for i in 0..Self::RETRY_ATTEMPTS {
            // Wait before trying again
            if i > 0 {
                eprintln!(
                    "Going to sleep for {} seconds before retrying discord",
                    Self::INTERVAL_BETWEEN_RETRY.as_secs()
                );
                std::thread::sleep(Self::INTERVAL_BETWEEN_RETRY);
            }

            match send_blocking_reqwest(msg, &discord.url) {
                Ok(()) => return Ok(()),
                Err(e) => eprintln!(
                    "attempt #{} failed to send via discord with msg: {e:?}",
                    i + 1
                ),
            }
        }
        bail!(
            "failed to send via discord after {} attempts",
            Self::RETRY_ATTEMPTS
        )
    }
}

fn send_blocking_reqwest(msg: &str, url: &str) -> anyhow::Result<()> {
    let resp = reqwest::blocking::Client::new()
        .request(reqwest::Method::POST, url)
        .header("Content-Type", "application/json")
        .body(json!({ "content": msg }).to_string())
        .send()?;
    if resp.status().is_success() {
        Ok(())
    } else {
        let unexpected_response_value =
            resp.error_for_status().context("error response returned")?;

        // Assumption the code above will always result in an error being returned
        bail!("not a successful status but conversion to error still failed? Response: {unexpected_response_value:?}")
    }
}
