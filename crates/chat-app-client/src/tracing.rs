#[cfg(not(target_arch = "wasm32"))]
pub fn init(cli: &super::cli::Cli) -> anyhow::Result<()> {
    use anyhow::bail;
    use wykies_shared::telemetry;

    fn init_to_file() -> anyhow::Result<()> {
        let (file, filename) = telemetry::create_trace_file("chat_app_client")?;
        let subscriber =
            telemetry::get_subscriber("chat_app_client".into(), "zbus=warn,info", file);

        // Start logging to file
        match telemetry::init_subscriber(subscriber) {
            Ok(_) => {
                println!("Tracing started to file {filename:?}");
                Ok(())
            }
            Err(e) => {
                bail!("Failed to start tracing to file. Error: {e}");
            }
        }
    }

    if !cli.is_to_std_out {
        // Log to file
        match init_to_file() {
            Ok(_) => return Ok(()),
            Err(e) => {
                // Print error and fall though to logging to stdout
                eprintln!("Failed to start logging to file: {e}");
            }
        }
    }

    // Log to stdout
    match tracing_subscriber::fmt().try_init() {
        Ok(_) => Ok(()),
        Err(e) => {
            bail!("Failed to start tracing. Error: {e}");
        }
    }
}
