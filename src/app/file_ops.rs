use crate::app::state::HexApp;
use crate::file_buffer::FileBuffer;

impl HexApp {
    pub fn open_file(&mut self, slot_index: usize) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            match FileBuffer::from_path(path) {
                Ok(buf) => self.files[slot_index] = Some(buf),
                Err(e) => eprintln!("Error opening file: {}", e),
            }
        }
    }

    pub fn save_file(&mut self, slot_index: usize) {
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

    pub fn close_file(&mut self, slot_index: usize) {
        self.files[slot_index] = None;
        if slot_index == 0 && self.files[1].is_some() {
            self.files[0] = self.files[1].take();
        }
        if let Some((c_idx, _)) = self.cursor {
            if c_idx == slot_index {
                self.cursor = None;
            }
        }
    }
}
