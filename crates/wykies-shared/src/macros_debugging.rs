/// Logs the message passed then panics if running debug and panic on rare
/// errors is enabled or does nothing not even logging to make it safe to use in
/// loops if it will not panic
#[macro_export]
macro_rules! debug_panic {
    ($($arg:tt)*) => {{
        #[cfg(debug_assertions)]
        if wykies_shared::const_config::PANIC_ON_RARE_ERR {
            let err_msg = format!($($arg)*);
            tracing::error!(?err_msg, "DEBUG PANIC!!!");
            panic!(
                "Rare error detected! Panicking to make it more obvious: {}",
                err_msg
            )
        }
    }};
}

/// Returns an error message with the position in the code included
///
/// Note: Unable to log message as it's too easy for this to end up in the UI
/// loop of the client and fill up the HDD with the log
#[macro_export]
macro_rules! internal_error_msg {
    ($($arg:tt)*) => {{
        let res = format!($($arg)*);
        let internal_error_msg = format!(
            "{}\ninternal error: {}:{}:{}",
            res,
            file!(),
            line!(),
            column!()
        );
        internal_error_msg
    }};
}

/// Logs the contents passed as an error and forwards it on to debug panic
#[macro_export]
macro_rules! log_as_error {
    ($($arg:tt)*) => {{
        let err_msg = format!($($arg)*);
        tracing::error!(?err_msg, "LOGGED AS ERROR");
        wykies_shared::debug_panic!("{err_msg:?}");
    }};
}

/// Use this version if we don't want to crash during prod but we do want to
/// crash during development and log if it was an error either way
#[macro_export]
macro_rules! log_err_as_error {
    ($arg: expr) => {
        if let Err(err_msg) = $arg {
            tracing::error!(?err_msg, "ERROR VARIANT FOUND AND IS BEING LOGGED");
            wykies_shared::debug_panic!("{err_msg:?}");
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
