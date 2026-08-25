#[expect(
    clippy::print_stdout,
    reason = "really helps to easily be able to see where the traces are going to"
)]
#[cfg(not(target_arch = "wasm32"))]
pub fn init() -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
    use anyhow::{Context as _, bail};
    use wykies_shared::telemetry;

    let (writer, path, guard) = telemetry::setup_tracing_writer("chat_app_client")?;
    let subscriber = telemetry::get_subscriber("chat_app_client".into(), "zbus=warn,info", writer);

    match telemetry::init_subscriber(subscriber) {
        Ok(()) => {
            println!(
                "Traces being written to: {:?}",
                path.canonicalize()
                    .context("trace file canonicalization failed")?
            );
            Ok(guard)
        }
        Err(e) => {
            bail!("Failed to start tracing to file. Error: {e}");
        }
    }
}
