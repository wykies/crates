mod cli;
mod log_info;
mod notification;
mod state;

use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context};
pub use cli::Cli;
pub use log_info::LogInfo;
use notification::send_notification;
pub use state::AppState;

pub fn run(cli: &Cli) -> anyhow::Result<()> {
    let (state_file, config_folder) = get_canonical_folder_and_filename(&cli.state_file)?;
    let mut app_state = AppState::load(state_file).context("failed to load state")?;

    if cli.print_state_only {
        println!("{app_state:#?}");
        return Ok(());
    }

    if let Some(msg) = &cli.test_notification {
        send_notification(msg, &config_folder).context("sending test notification failed")?;
        println!("TEST NOTIFICATION SENT");
        return Ok(());
    }

    if app_state.alive_msg_due() {
        let alive_msg = app_state.generate_alive_msg();
        send_notification(&alive_msg, &config_folder).context("failed to send alive message")?;
    }

    match process_logs_folder(&mut app_state).context("error processing logs") {
        Ok(log_infos) => {
            if !log_infos.is_empty() {
                let err_msg = build_err_msg_from_logs(log_infos);
                send_notification(&err_msg, &config_folder)
                    .context("failed to send notification of errors")?
            }
        }
        Err(e) => send_notification(&e.to_string(), &config_folder)
            .context("failed to send notification of processing failure")?,
    }

    if let Some(msg) = app_state.generate_inactivity_msg() {
        send_notification(&msg, &config_folder)
            .context("failed to send notification of inactivity in logs")?
    }

    if app_state.is_changed() {
        app_state
            .save(&cli.state_file)
            .context("failed to save state")?;
    }
    println!("RUN COMPLETED");
    Ok(())
}

pub fn build_err_msg_from_logs(log_infos: Vec<LogInfo>) -> String {
    const MAX_MSG_LEN: usize = 2000;
    let log_count = log_infos.len();
    // Stores the running count of `Errors and warnings` found
    let mut entry_count = 0;
    let mut result = String::new();
    let separator = "---\n";
    for log_info in log_infos {
        entry_count += log_info.errors_and_warnings.len();
        result.push_str(separator);
        result.push_str(&log_info.to_string());
    }
    result.push_str(separator);
    let summary = format!("{log_count} logs with {entry_count} error and warnings\n");
    result.push_str(&summary);
    result.push_str(separator);
    if result.len() > MAX_MSG_LEN {
        // Exceeded limit for discord, only send summary
        result = summary;
        result.push_str(separator);
        result.push_str("DETAILS TOO LONG TO INCLUDE **TRUNCATED**");
        result.push_str(separator);
    }
    result
}

/// Returns a list of the new logs with errors
pub fn process_logs_folder(app_state: &mut AppState) -> anyhow::Result<Vec<LogInfo>> {
    let mut result = Vec::new();
    let mut latest_timestamp = app_state.latest_log_datetime();
    for dir_entry in read_dir(app_state.logs_dir())
        .with_context(|| format!("failed to read log folder: {:?}", app_state.logs_dir()))?
    {
        let dir_entry = dir_entry.with_context(|| {
            format!("failed to read entry in folder: {:?}", app_state.logs_dir())
        })?;

        if dir_entry
            .file_type()
            .with_context(|| format!("failed to get file type for: {:?}", dir_entry.path()))?
            .is_file()
        {
            let mut log_info = LogInfo::new(dir_entry.file_name().to_string_lossy())?;
            if log_info.date_time > app_state.latest_log_datetime() {
                if log_info.date_time > latest_timestamp {
                    // Save latest timestamp found
                    latest_timestamp = log_info.date_time;
                }
                if log_info.abnormal_outcome.is_some() {
                    log_info.extract_errors(&dir_entry.path())?;
                    result.push(log_info);
                }
            }
        }
    }
    if latest_timestamp > app_state.latest_log_datetime() {
        app_state.set_latest_log_datetime(latest_timestamp);
    }

    // Sort output to show errors in age order
    result.sort_by_key(|x| x.date_time);

    Ok(result)
}

/// Converts the input into it's canonical form and based on the assumption that it is a file also returns the parent folder
fn get_canonical_folder_and_filename<P: AsRef<Path>>(
    file_path: P,
) -> anyhow::Result<(PathBuf, PathBuf)> {
    let canonical_file_path = file_path.as_ref().canonicalize().with_context(|| {
        format!(
            "failed to get canonical version of: {:?} in working directory: {:?}",
            file_path.as_ref(),
            match std::env::current_dir() {
                Ok(cwd) => cwd.to_string_lossy().to_string(),
                Err(e) => format!("[failed {e}]"),
            }
        )
    })?;
    let parent_folder = canonical_file_path
        .parent()
        .ok_or(anyhow!(
            "failed to get parent folder for file: {}",
            canonical_file_path.display()
        ))?
        .to_path_buf();
    Ok((canonical_file_path, parent_folder))
}

pub fn init_state<P: AsRef<Path>>(logs: P, state_file: P) -> anyhow::Result<()> {
    let mut state = AppState::new(logs.as_ref().to_path_buf());
    state.save(state_file)
}
