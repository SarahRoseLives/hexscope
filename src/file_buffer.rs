use std::fs;
use std::path::PathBuf;

pub struct FileBuffer {
    pub path: Option<PathBuf>,
    pub data: Vec<u8>,
    pub dirty: bool,
}

impl FileBuffer {
    pub fn from_path(path: PathBuf) -> std::io::Result<Self> {
        let data = fs::read(&path)?;
        Ok(Self {
            path: Some(path),
            data,
            dirty: false,
        })
    }

    #[allow(dead_code)] // Useful for future "New File" feature
    pub fn new_empty() -> Self {
        Self {
            path: None,
            data: Vec::new(),
            dirty: false,
        }
    }
}