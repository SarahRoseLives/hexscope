use crate::app::state::HexApp;
use eframe::egui;

impl eframe::App for HexApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_input(ctx);

        ctx.style_mut(|style| {
            style.override_text_style = Some(egui::TextStyle::Monospace);
            style.visuals.dark_mode = true;
        });

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.render_top_bar(ui);
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            self.render_bottom_bar(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let has_f1 = self.files[0].is_some();
            let has_f2 = self.files[1].is_some();

            if !has_f1 && !has_f2 {
                ui.centered_and_justified(|ui| {
                    ui.heading("Open a file to begin");
                });
                return;
            }

            let scroll_arg = if self.sync_scroll { Some(self.common_scroll_offset) } else { None };

            if has_f1 && has_f2 {
                ui.columns(2, |columns| {
                    let s1 = self.render_hex_pane(&mut columns[0], 0, "view_left", scroll_arg);
                    let s2 = self.render_hex_pane(&mut columns[1], 1, "view_right", scroll_arg);

                    if self.sync_scroll {
                        if let Some(o) = s1 { self.common_scroll_offset = o; }
                        else if let Some(o) = s2 { self.common_scroll_offset = o; }
                    }
                });
            } else {
                let active_slot = if has_f1 { 0 } else { 1 };
                let salt = if has_f1 { "view_left" } else { "view_right" };
                self.render_hex_pane(ui, active_slot, salt, None);
            }
        });
    }
}
