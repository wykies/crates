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
