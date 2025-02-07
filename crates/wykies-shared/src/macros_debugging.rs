#[macro_export]
macro_rules! debug_panic {
    ($arg: expr) => {
        #[cfg(debug_assertions)]
        if wykies_shared::const_config::PANIC_ON_RARE_ERR {
            panic!(
                "Rare error detected! Panicking to make it more obvious: {:?}",
                $arg
            )
        }
    };
}

/// Logs the error and includes the line in the code in the error message but
/// does NOT panic
#[macro_export]
macro_rules! internal_error {
    ($($arg:tt)*) => {{
        let res = format!($($arg)*);
        let internal_error_msg = format!(
            "{}\ninternal error: {}:{}:{}",
            res,
            file!(),
            line!(),
            column!()
        );
        // tracing::error!(?internal_error_msg);
        // TODO 1: Make this client only and make it panic instead
        internal_error_msg
    }};
}

/// Use this version if we don't want to crash during prod but we do want to
/// crash during development and log if it was an error either way
#[macro_export]
macro_rules! log_err_as_error {
    ($arg: expr) => {
        if let Err(err) = $arg {
            tracing::error!(?err);
            wykies_shared::debug_panic!(err);
        }
    };
}

/// Use this version if we know that under normal operation this can happen but
/// we wish to monitor it
#[macro_export]
macro_rules! log_err_as_warn {
    ($arg: expr) => {
        if let Err(mishap) = $arg {
            tracing::warn!(?mishap);
        }
    };
}
