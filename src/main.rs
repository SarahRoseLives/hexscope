mod app;
mod file_buffer;

use crate::app::state::HexApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "hexscope",
        options,
        Box::new(|_cc| Ok(Box::new(HexApp::default()))),
    )
}