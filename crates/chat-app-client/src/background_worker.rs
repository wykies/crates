#[cfg(not(target_arch = "wasm32"))]
pub fn create_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Unable to create Runtime")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn start_background_worker(rt: tokio::runtime::Runtime) {
    // Execute the runtime in its own thread.
    std::thread::spawn(move || {
        tracing::info!("Background worker started");
        rt.block_on(async {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        })
    });
}
