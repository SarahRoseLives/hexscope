use crate::app::state::{EditMode, HexApp};
use eframe::egui;

const BYTES_PER_ROW: usize = 16;

impl HexApp {
    pub fn render_top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("hexscope");
            ui.separator();

            if ui.button("📂 File 1").clicked() { self.open_file(0); }
            if self.files[0].as_ref().map_or(false, |f| f.dirty) {
                if ui.button("💾 Save 1").clicked() { self.save_file(0); }
            }
            if self.files[0].is_some() && ui.button("❌").clicked() { self.close_file(0); }

            ui.separator();

            if ui.button("📂 File 2").clicked() { self.open_file(1); }
            if self.files[1].as_ref().map_or(false, |f| f.dirty) {
                if ui.button("💾 Save 2").clicked() { self.save_file(1); }
            }
            if self.files[1].is_some() && ui.button("❌").clicked() { self.close_file(1); }

            ui.separator();
            ui.checkbox(&mut self.sync_scroll, "🔗 Sync");

            ui.separator();

            // Search
            ui.label("🔍");
            let s_resp = ui.add(egui::TextEdit::singleline(&mut self.search_query).desired_width(100.0).hint_text("Search..."));
            if s_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.perform_search();
            }
            if ui.button("Go").clicked() { self.perform_search(); }
            ui.checkbox(&mut self.search_hex_mode, "Hex");
            if !self.search_result_msg.is_empty() {
                ui.label(egui::RichText::new(&self.search_result_msg).size(10.0).weak());
            }

            ui.separator();

            // Jump
            ui.label("Px");
            let j_resp = ui.add(egui::TextEdit::singleline(&mut self.jump_offset_str).desired_width(60.0).hint_text("Offset"));
            if j_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.perform_jump();
            }
        });
    }

    pub fn render_bottom_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some((idx, offset)) = self.cursor {
                if let Some(file) = &self.files[idx] {
                    if offset < file.data.len() {
                        let val = file.data[offset];
                        let fname = file.path.as_ref()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy())
                            .unwrap_or("Untitled".into());

                        ui.label(egui::RichText::new(format!("File: {}", fname)).strong());
                        ui.separator();
                        ui.label(format!("Offset: 0x{:08X} ({})", offset, offset));
                        ui.separator();
                        let char_display = if val >= 32 && val <= 126 { val as char } else { '.' };
                        ui.label(format!("Value: 0x{:02X} ({}) '{}'", val, val, char_display));
                        ui.separator();
                        ui.label(format!("Mode: {}", if self.edit_mode == EditMode::Hex { "HEX" } else { "ASCII" }));
                    }
                }
            } else {
                ui.label("Ready");
            }
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_hex_pane(
        &mut self,
        ui: &mut egui::Ui,
        file_index: usize,
        id_salt: &str,
        force_scroll: Option<f32>
    ) -> Option<f32> {

        let (bytes_len, dirty, path_display) = if let Some(f) = &self.files[file_index] {
            (
                f.data.len(),
                f.dirty,
                f.path.as_ref().map(|p| p.to_string_lossy().into_owned()).unwrap_or("Untitled".into())
            )
        } else {
            return None;
        };

        let header = format!("{} {}{}",
            if file_index == 0 { "File 1:" } else { "File 2:" },
            path_display,
            if dirty { " (*)" } else { "" }
        );
        ui.heading(header);
        ui.separator();

        let text_style = egui::TextStyle::Monospace;
        let row_height = ui.text_style_height(&text_style);
        let total_rows = (bytes_len + BYTES_PER_ROW - 1) / BYTES_PER_ROW;

        let mut scroll_area = egui::ScrollArea::vertical().id_salt(id_salt).auto_shrink([false; 2]);
        if let Some(offset) = force_scroll {
            scroll_area = scroll_area.vertical_scroll_offset(offset);
        }

        let output = scroll_area.show_rows(ui, row_height, total_rows, |ui, row_range| {
            egui::Grid::new(id_salt)
                .striped(true)
                .spacing([15.0, 0.0])
                .min_col_width(0.0)
                .show(ui, |ui| {
                    for row in row_range {
                        self.render_row(ui, file_index, row);
                    }
                });
        });

        if let Some(forced) = force_scroll {
            if (output.state.offset.y - forced).abs() > 0.1 {
                return Some(output.state.offset.y);
            }
        } else {
            return Some(output.state.offset.y);
        }
        None
    }

    fn render_row(&mut self, ui: &mut egui::Ui, file_index: usize, row: usize) {
        let offset = row * BYTES_PER_ROW;

        let data = &self.files[file_index].as_ref().unwrap().data;
        let other_index = if file_index == 0 { 1 } else { 0 };
        let compare_data = self.files[other_index].as_ref().map(|f| &f.data);

        // 1. Offset Column
        ui.label(egui::RichText::new(format!("{:08X}", offset)).weak());

        let range_end = (offset + BYTES_PER_ROW).min(data.len());
        let chunk = &data[offset..range_end];

        let compare_chunk = compare_data.and_then(|d| {
            if offset >= d.len() { None } else {
                let end = (offset + BYTES_PER_ROW).min(d.len());
                Some(&d[offset..end])
            }
        });

        // 2. Hex View
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for (i, &byte) in chunk.iter().enumerate() {
                let abs_idx = offset + i;
                let mut text = egui::RichText::new(format!("{:02X}", byte));

                // Diff highlighting
                if let Some(comp) = compare_chunk {
                    if i < comp.len() && comp[i] != byte {
                        text = text.color(egui::Color32::LIGHT_RED).strong();
                    }
                }

                // Selection Highlighting (Hex Mode)
                if self.cursor == Some((file_index, abs_idx)) {
                    // Use Blue for Hex Focus
                    let bg = if self.edit_mode == EditMode::Hex { egui::Color32::DARK_BLUE } else { egui::Color32::from_gray(60) };
                    text = text.background_color(bg).color(egui::Color32::WHITE);
                    if self.edit_mode == EditMode::Hex && self.cursor_low_nibble {
                        text = text.underline();
                    }
                }

                if i == 8 { ui.add_space(4.0); }

                let resp = ui.add(egui::Label::new(text).sense(egui::Sense::click()));
                if resp.clicked() {
                    self.cursor = Some((file_index, abs_idx));
                    self.edit_mode = EditMode::Hex; // Switch to Hex Mode
                    self.cursor_low_nibble = false;
                }
            }
        });

        // 3. ASCII View
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add(egui::Separator::default().vertical());
            for (i, &byte) in chunk.iter().enumerate() {
                let char_display = if byte >= 32 && byte <= 126 { byte as char } else { '.' };
                let mut text = egui::RichText::new(char_display.to_string());

                if let Some(comp) = compare_chunk {
                    if i < comp.len() && comp[i] != byte {
                        text = text.color(egui::Color32::LIGHT_RED);
                    }
                }

                let abs_idx = offset + i;
                if self.cursor == Some((file_index, abs_idx)) {
                    // Use Green for ASCII Focus to differentiate
                    let bg = if self.edit_mode == EditMode::Ascii { egui::Color32::DARK_GREEN } else { egui::Color32::from_gray(60) };
                    text = text.background_color(bg).color(egui::Color32::WHITE);
                }

                // Clickable ASCII
                let resp = ui.add(egui::Label::new(text).sense(egui::Sense::click()));
                if resp.clicked() {
                    self.cursor = Some((file_index, abs_idx));
                    self.edit_mode = EditMode::Ascii; // Switch to ASCII Mode
                }
            }
        });

        ui.end_row();
    }
}


