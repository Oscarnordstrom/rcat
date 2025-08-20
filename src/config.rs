/// Configuration constants for the application
pub struct Config;

impl Config {
    /// Maximum size of content to copy to clipboard
    pub const MAX_SIZE: usize = 1 * 1024 * 1024 * 1024;

    /// Maximum number of worker threads
    pub const MAX_THREADS: usize = 4;

    /// Buffer size for binary file detection
    pub const BINARY_CHECK_BUFFER_SIZE: usize = 8192;
}
