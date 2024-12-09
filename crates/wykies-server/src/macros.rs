/// Use this version if we don't know any reason why this might happen under
/// normal circumstances but it's not worth crashing for and execution can go on
#[macro_export]
macro_rules! log_err_as_error {
    ($arg: expr) => {
        if let Err(err) = $arg {
            tracing::error!(?err);
            debug_panic!(err);
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
