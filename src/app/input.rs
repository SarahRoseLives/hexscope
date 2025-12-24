use crate::app::state::{EditMode, HexApp};
use eframe::egui;

impl HexApp {
    pub fn handle_input(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            return;
        }

        if let Some((idx, offset)) = self.cursor {
            if let Some(file) = &mut self.files[idx] {
                if offset >= file.data.len() {
                    return;
                }

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
}
