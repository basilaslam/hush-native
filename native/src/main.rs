#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
mod audio;
mod brand;
mod config;
mod coordination;
mod engine;
mod integration;
mod model;
mod transport;
mod ui;
mod updater;
fn main() -> eframe::Result {
    let _ = rustls::crypto::ring::default_provider().install_default();
    if std::env::args().any(|arg| arg == "--self-test") {
        let runtime =
            tokio::runtime::Runtime::new().expect("Could not start the self-test runtime");
        runtime.block_on(integration::native_end_to_end());
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Hush · Your space to play")
            .with_icon(brand::window_icon())
            .with_inner_size([1300., 880.])
            .with_min_inner_size([500., 650.]),
        centered: true,
        renderer: eframe::Renderer::Glow,
        vsync: true,
        ..Default::default()
    };
    eframe::run_native(
        "Hush",
        options,
        Box::new(|cc| Ok(Box::new(ui::HushApp::new(cc)?))),
    )
}
