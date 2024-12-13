#[macro_export]
macro_rules! debug_panic {
    ($arg: expr) => {
        if cfg!(debug_assertions) && wykies_shared::const_config::PANIC_ON_RARE_ERR {
            panic!(
                "Rare error detected! Panicking to make it more obvious: {:?}",
                $arg
            )
        }
    };
}

#[macro_export]
macro_rules! internal_error {
    ($arg: expr) => {{
        let internal_error_msg = format!(
            "{}\ninternal error: {}:{}:{}",
            $arg,
            file!(),
            line!(),
            column!()
        );
        tracing::error!(?internal_error_msg);
        internal_error_msg
    }};
}

/// Use this version if we don't know any reason why this might happen under
/// normal circumstances but it's not worth crashing for and execution can go on
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
