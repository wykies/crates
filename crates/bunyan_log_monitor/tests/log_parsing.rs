use std::path::{Path, PathBuf};

use chrono::Local;
use fs_log_monitor::{build_err_msg_from_logs, process_logs_folder, AppState};

fn samples_folder() -> PathBuf {
    Path::new("tests").join("sample_logs")
}

#[test]
fn no_files_pass_filter() {
    let before_app_state_created = Local::now().naive_local();
    let mut actual = AppState::new(samples_folder());
    let expected = actual.clone();
    assert!(
        actual.latest_log_datetime() >= before_app_state_created,
        "date assumed to be now or later so that the sample logs from the past should not be included"
    );

    let log_infos = process_logs_folder(&mut actual).unwrap();
    assert!(
        log_infos.is_empty(),
        "all samples should be in the past and filtered out"
    );

    assert_eq!(actual, expected, "no changes expected to AppState created")
}

#[test]
fn output_snapshot() {
    let mut app_state = AppState::new_with_min_dates(samples_folder());

    let logs_infos = process_logs_folder(&mut app_state).unwrap();
    insta::assert_ron_snapshot!(logs_infos);
    insta::assert_ron_snapshot!(app_state, {
        ".last_alive_msg" => "date_time",
    });

    let msg = build_err_msg_from_logs(logs_infos);
    insta::assert_snapshot!(msg);
}
