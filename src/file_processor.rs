use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::config::Config;

/// Result of processing a file
#[derive(Debug)]
pub enum FileContent {
    /// Text file with its content
    Text(String),
    /// Binary file marker
    Binary,
    /// File that couldn't be read
    Unreadable,
}

/// Processes a file and returns its content or type
pub struct FileProcessor;

impl FileProcessor {
    /// Process a file at the given path
    pub fn process(path: &Path) -> FileContent {
        if Self::is_binary(path) {
            FileContent::Binary
        } else {
            match std::fs::read_to_string(path) {
                Ok(content) => FileContent::Text(content),
                Err(_) => FileContent::Unreadable,
            }
        }
    }

    /// Check if a file is binary by looking for null bytes
    pub fn is_binary(path: &Path) -> bool {
        let mut file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return false,
        };

        let mut buffer = vec![0u8; Config::BINARY_CHECK_BUFFER_SIZE];

        match file.read(&mut buffer) {
            Ok(bytes_read) => buffer[..bytes_read].contains(&0),
            Err(_) => false,
        }
    }

    /// Format file content for output
    pub fn format_content(path: &Path, content: FileContent) -> Option<String> {
        match content {
            FileContent::Text(text) => Some(format!("--- {} ---\n{}", path.display(), text)),
            FileContent::Binary => Some(format!("--- {} ---\n<BINARY_FILE>", path.display())),
            FileContent::Unreadable => None,
        }
    }
}
