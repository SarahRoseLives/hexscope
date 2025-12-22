use eframe::egui;
use crate::file_buffer::FileBuffer;

const BYTES_PER_ROW: usize = 16;

#[derive(PartialEq, Clone, Copy)]
enum EditMode {
    Hex,
    Ascii,
}

pub struct HexApp {
    // index 0 = Left/Main, index 1 = Right/Diff
    files: [Option<FileBuffer>; 2],

    // UI State
    sync_scroll: bool,
    common_scroll_offset: f32,

    // Selection / Editing
    cursor: Option<(usize, usize)>, // (file_index, byte_offset)
    cursor_low_nibble: bool,        // For Hex editing only
    edit_mode: EditMode,            // Hex or Ascii?

    // Search
    search_query: String,
    search_hex_mode: bool,
    search_result_msg: String,

    // Jump
    jump_offset_str: String,
}

impl Default for HexApp {
    fn default() -> Self {
        Self {
            files: [None, None],
            sync_scroll: true,
            common_scroll_offset: 0.0,
            cursor: None,
            cursor_low_nibble: false,
            edit_mode: EditMode::Hex,
            search_query: String::new(),
            search_hex_mode: false,
            search_result_msg: String::new(),
            jump_offset_str: String::new(),
        }
    }
}

impl HexApp {
    // --- Input Handling ---

    fn handle_input(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() { return; }

        if let Some((idx, offset)) = self.cursor {
            if let Some(file) = &mut self.files[idx] {
                if offset >= file.data.len() { return; }

                // Collect text input
                let input_events = ctx.input(|i| i.events.clone());

                for event in input_events {
                    if let egui::Event::Text(text) = event {
                        match self.edit_mode {
                            // --- HEX MODE EDITING ---
                            EditMode::Hex => {
                                for c in text.chars() {
                                    if let Some(val) = c.to_digit(16) {
                                        let val = val as u8;
                                        let current_byte = file.data[offset];

                                        if !self.cursor_low_nibble {
                                            // Set High Nibble
                                            file.data[offset] = (val << 4) | (current_byte & 0x0F);
                                            self.cursor_low_nibble = true;
                                        } else {
                                            // Set Low Nibble
                                            file.data[offset] = (current_byte & 0xF0) | val;
                                            self.cursor_low_nibble = false;
                                            // Advance cursor
                                            if offset + 1 < file.data.len() {
                                                self.cursor = Some((idx, offset + 1));
                                            }
                                        }
                                        file.dirty = true;
                                    }
                                }
                            }
                            // --- ASCII MODE EDITING ---
                            EditMode::Ascii => {
                                for c in text.chars() {
                                    // Only allow valid bytes, ignore control chars if necessary
                                    // but usually we want to allow typing anything that maps to a byte
                                    let byte_val = c as u8; // Basic truncation for UTF-8 -> u8

                                    file.data[offset] = byte_val;
                                    file.dirty = true;

                                    // Auto-advance
                                    if offset + 1 < file.data.len() {
                                        self.cursor = Some((idx, offset + 1));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // --- File Operations ---

    fn open_file(&mut self, slot_index: usize) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            match FileBuffer::from_path(path) {
                Ok(buf) => self.files[slot_index] = Some(buf),
                Err(e) => eprintln!("Error opening file: {}", e),
            }
        }
    }

    fn save_file(&mut self, slot_index: usize) {
        if let Some(file) = &mut self.files[slot_index] {
            if let Some(path) = &file.path {
                if let Err(e) = std::fs::write(path, &file.data) {
                    eprintln!("Error saving file: {}", e);
                } else {
                    file.dirty = false;
                }
            }
        }
    }

    fn close_file(&mut self, slot_index: usize) {
        self.files[slot_index] = None;
        if slot_index == 0 && self.files[1].is_some() {
            self.files[0] = self.files[1].take();
        }
        if let Some((c_idx, _)) = self.cursor {
            if c_idx == slot_index { self.cursor = None; }
        }
    }

    // --- Search & Jump ---

    fn perform_search(&mut self) {
        let target_idx = self.cursor.map(|(i, _)| i).unwrap_or(0);
        let start_offset = self.cursor.map(|(_, off)| off + 1).unwrap_or(0);

        if let Some(file) = &self.files[target_idx] {
            let needle: Vec<u8> = if self.search_hex_mode {
                let cleaned: String = self.search_query.chars().filter(|c| c.is_ascii_hexdigit()).collect();
                cleaned.as_bytes().chunks(2)
                    .map(|c| {
                        let s = std::str::from_utf8(c).unwrap_or("00");
                        u8::from_str_radix(s, 16).unwrap_or(0)
                    })
                    .collect()
            } else {
                self.search_query.as_bytes().to_vec()
            };

            if needle.is_empty() { return; }

            let found = file.data[start_offset..]
                .windows(needle.len())
                .position(|w| w == needle)
                .map(|p| p + start_offset)
                .or_else(|| {
                    file.data[..start_offset]
                        .windows(needle.len())
                        .position(|w| w == needle)
                });

            match found {
                Some(offset) => {
                    self.jump_to_offset(offset);
                    self.cursor = Some((target_idx, offset));
                    self.search_result_msg = format!("Found at {:X}", offset);
                }
                None => {
                    self.search_result_msg = "Not found".to_string();
                }
            }
        }
    }

    fn perform_jump(&mut self) {
        let clean = self.jump_offset_str.trim();
        let offset = if clean.starts_with("0x") || clean.starts_with("0X") {
            usize::from_str_radix(&clean[2..], 16)
        } else {
            clean.parse::<usize>()
        };

        if let Ok(off) = offset {
            self.jump_to_offset(off);
            let target_idx = self.cursor.map(|(i, _)| i).unwrap_or(0);
            self.cursor = Some((target_idx, off));
        }
    }

    fn jump_to_offset(&mut self, offset: usize) {
        let row = offset / BYTES_PER_ROW;
        let estimated_y = (row as f32) * 18.0;
        self.common_scroll_offset = estimated_y;
    }

    // --- Rendering Helpers ---

    fn render_top_bar(&mut self, ui: &mut egui::Ui) {
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

    fn render_bottom_bar(&self, ui: &mut egui::Ui) {
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
    fn render_hex_pane(
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