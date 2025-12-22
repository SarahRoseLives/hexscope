mod file_buffer;
mod hex_app;

use eframe::egui;
use hex_app::HexApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "hexscope",
        options,
        Box::new(|_cc| Ok(Box::new(HexApp::default()))),
    )
}