use crate::file_buffer::FileBuffer;

#[derive(PartialEq, Clone, Copy)]
pub enum EditMode {
    Hex,
    Ascii,
}

pub struct HexApp {
    // index 0 = Left/Main, index 1 = Right/Diff
    pub files: [Option<FileBuffer>; 2],

    // UI State
    pub sync_scroll: bool,
    pub common_scroll_offset: f32,

    // Selection / Editing
    pub cursor: Option<(usize, usize)>, // (file_index, byte_offset)
    pub cursor_low_nibble: bool,        // For Hex editing only
    pub edit_mode: EditMode,            // Hex or Ascii?

    // Search
    pub search_query: String,
    pub search_hex_mode: bool,
    pub search_result_msg: String,

    // Jump
    pub jump_offset_str: String,
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
