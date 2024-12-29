#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use clap::Parser;
    let args = chat_app_client::cli::Cli::parse();

    if let Err(e) = chat_app_client::tracing::init(&args) {
        eprintln!("Failed to start tracing: {e}");
    }

    let rt = chat_app_client::background_worker::create_runtime();
    let _enter = rt.enter(); // This Guard must be held to call `tokio::spawn` anywhere in the program
    chat_app_client::background_worker::start_background_worker(rt); // This is also needed to prevent the runtime from stopping

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // TODO 4: Fix default size of chat client
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "ChatApp",
        native_options,
        Box::new(|cc| Ok(Box::new(chat_app_client::ChatApp::new(cc)))),
    )
}

// When compiling to web using trunk
#[cfg(target_arch = "wasm32")]
fn main() {
    // TODO 4: Look into dark mode not working in WASM
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(chat_app_client::wasm_log_level()).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window found")
            .document()
            .expect("No document found (No DOM)");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(chat_app_client::ChatApp::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
