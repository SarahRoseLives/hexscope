use crate::app::state::HexApp;

const BYTES_PER_ROW: usize = 16;

impl HexApp {
    pub fn perform_search(&mut self) {
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

            if needle.is_empty() {
                return;
            }

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

    pub fn perform_jump(&mut self) {
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
}
