use eframe::egui;
use std::fs;
use std::path::PathBuf;

// Configuration
const BYTES_PER_ROW: usize = 16;

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

struct FileBuffer {
    path: Option<PathBuf>,
    data: Vec<u8>,
    dirty: bool,
}

impl FileBuffer {
    fn from_path(path: PathBuf) -> std::io::Result<Self> {
        let data = fs::read(&path)?;
        Ok(Self { path: Some(path), data, dirty: false })
    }
}

struct HexApp {
    // index 0 = Left/Main, index 1 = Right/Diff
    files: [Option<FileBuffer>; 2],

    // UI State
    sync_scroll: bool,
    common_scroll_offset: f32,

    // Selection / Editing
    // (file_index, byte_offset)
    cursor: Option<(usize, usize)>,
    // If true, we are editing the lower nibble (second digit).
    // If false, we are editing the upper nibble (first digit).
    cursor_low_nibble: bool,

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
            search_query: String::new(),
            search_hex_mode: false,
            search_result_msg: String::new(),
            jump_offset_str: String::new(),
        }
    }
}

impl HexApp {
    // --- File Ops ---

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
                if let Err(e) = fs::write(path, &file.data) {
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
    }

    // --- Core Logic: Search & Jump ---

    fn perform_search(&mut self) {
        // We always search in the "active" file (where the cursor is), or default to File 0
        let target_idx = self.cursor.map(|(i, _)| i).unwrap_or(0);
        let start_offset = self.cursor.map(|(_, off)| off + 1).unwrap_or(0); // Start AFTER current cursor

        if let Some(file) = &self.files[target_idx] {
            let needle: Vec<u8> = if self.search_hex_mode {
                // Parse hex string "DEAD" -> [0xDE, 0xAD]
                let cleaned: String = self.search_query.chars().filter(|c| c.is_ascii_hexdigit()).collect();
                cleaned
                    .as_bytes()
                    .chunks(2)
                    .map(|chunk| {
                        let s = std::str::from_utf8(chunk).unwrap_or("00");
                        u8::from_str_radix(s, 16).unwrap_or(0)
                    })
                    .collect()
            } else {
                // ASCII Search
                self.search_query.as_bytes().to_vec()
            };

            if needle.is_empty() { return; }

            // Find first occurrence after start_offset
            let found = file.data[start_offset..]
                .windows(needle.len())
                .position(|window| window == needle)
                .map(|p| p + start_offset)
                // If not found, wrap around to beginning
                .or_else(|| {
                    file.data[..start_offset]
                        .windows(needle.len())
                        .position(|window| window == needle)
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
            // Move cursor there too
            let target_idx = self.cursor.map(|(i, _)| i).unwrap_or(0);
            self.cursor = Some((target_idx, off));
        }
    }

    fn jump_to_offset(&mut self, offset: usize) {
        let row = offset / BYTES_PER_ROW;
        // Approximation: Row height is ~14.0 (font size) + spacing.
        // We set the common scroll offset roughly to that row.
        // A more precise way requires calculating row height dynamically,
        // but for this structure, pure math works well enough.
        // We assume ~20px per row (font + spacing)
        let estimated_y = (row as f32) * 18.0; // 14pt monospace + padding
        self.common_scroll_offset = estimated_y;
    }

    // --- Input Handling ---

    fn handle_input(&mut self, ctx: &egui::Context) {
        // Only capture input if we are NOT typing in a text box (like search)
        if ctx.wants_keyboard_input() { return; }

        if let Some((idx, offset)) = self.cursor {
            if let Some(file) = &mut self.files[idx] {
                if offset >= file.data.len() { return; }

                let input = ctx.input(|i| {
                    i.events.iter().find_map(|e| {
                        if let egui::Event::Text(s) = e {
                            s.chars().next().filter(|c| c.is_ascii_hexdigit())
                        } else {
                            None
                        }
                    })
                });

                if let Some(c) = input {
                    let val = c.to_digit(16).unwrap() as u8;
                    let current_byte = file.data[offset];

                    if !self.cursor_low_nibble {
                        // Edit High Nibble: Replace top 4 bits, clear bottom?
                        // Or just overwrite top? Usually overwrite top, keep bottom.
                        // Formula: (val << 4) | (current & 0x0F)
                        file.data[offset] = (val << 4) | (current_byte & 0x0F);
                        self.cursor_low_nibble = true; // Move to low part
                    } else {
                        // Edit Low Nibble: Keep top, replace bottom
                        // Formula: (current & 0xF0) | val
                        file.data[offset] = (current_byte & 0xF0) | val;

                        // Advance Cursor!
                        self.cursor_low_nibble = false; // Reset for next byte
                        if offset + 1 < file.data.len() {
                            self.cursor = Some((idx, offset + 1));
                        }
                    }
                    file.dirty = true;
                }
            }
        }
    }

    // --- Rendering ---

    fn render_top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("hexscope");
            ui.separator();

            // File Controls
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

            // Search Block
            ui.label("🔍");
            let search_resp = ui.add(egui::TextEdit::singleline(&mut self.search_query).desired_width(100.0).hint_text("Search..."));
            if search_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.perform_search();
            }
            if ui.button("Go").clicked() { self.perform_search(); }
            ui.checkbox(&mut self.search_hex_mode, "Hex");
            if !self.search_result_msg.is_empty() {
                ui.label(egui::RichText::new(&self.search_result_msg).size(10.0).weak());
            }

            ui.separator();

            // Jump Block
            ui.label("Px");
            let jump_resp = ui.add(egui::TextEdit::singleline(&mut self.jump_offset_str).desired_width(60.0).hint_text("Offset"));
            if jump_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.perform_jump();
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

        // 1. Offset
        ui.label(egui::RichText::new(format!("{:08X}", offset)).weak());

        let range_end = (offset + BYTES_PER_ROW).min(data.len());
        let chunk = &data[offset..range_end];

        let compare_chunk = compare_data.and_then(|d| {
            if offset >= d.len() { None } else {
                let end = (offset + BYTES_PER_ROW).min(d.len());
                Some(&d[offset..end])
            }
        });

        // 2. Hex
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for (i, &byte) in chunk.iter().enumerate() {
                let abs_idx = offset + i;
                let mut text = egui::RichText::new(format!("{:02X}", byte));

                // Diff color
                if let Some(comp) = compare_chunk {
                    if i < comp.len() && comp[i] != byte {
                        text = text.color(egui::Color32::LIGHT_RED).strong();
                    }
                }

                // Selection / Cursor Logic
                if self.cursor == Some((file_index, abs_idx)) {
                     text = text.background_color(egui::Color32::DARK_BLUE).color(egui::Color32::WHITE);
                     if self.cursor_low_nibble {
                         text = text.underline(); // Underline to indicate nibble position (subtle hint)
                     }
                }

                if i == 8 { ui.add_space(4.0); }

                let resp = ui.add(egui::Label::new(text).sense(egui::Sense::click()));
                if resp.clicked() {
                    self.cursor = Some((file_index, abs_idx));
                    self.cursor_low_nibble = false; // Reset to start of byte on click
                }
            }
        });

        // 3. ASCII
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
                     text = text.background_color(egui::Color32::DARK_BLUE).color(egui::Color32::WHITE);
                }

                ui.label(text);
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
                // Split View
                ui.columns(2, |columns| {
                    let s1 = self.render_hex_pane(&mut columns[0], 0, "view_left", scroll_arg);
                    let s2 = self.render_hex_pane(&mut columns[1], 1, "view_right", scroll_arg);

                    if self.sync_scroll {
                        if let Some(o) = s1 { self.common_scroll_offset = o; }
                        else if let Some(o) = s2 { self.common_scroll_offset = o; }
                    }
                });
            } else {
                // Full View (Single File)
                let active_slot = if has_f1 { 0 } else { 1 };
                let salt = if has_f1 { "view_left" } else { "view_right" };
                self.render_hex_pane(ui, active_slot, salt, None);
            }
        });
    }
}