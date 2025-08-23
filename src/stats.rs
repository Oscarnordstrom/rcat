use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Statistics collector for tracking processing metrics
pub struct StatsCollector {
    files_processed: usize,
    directories_processed: usize,
    binary_files: usize,
    text_files: usize,
    unreadable_files: usize,
    skipped_files: usize,
    skipped_directories: usize,
    skipped_large_files: usize,
    gitignored_files: usize,
    gitignored_directories: usize,
    gitignore_files: Vec<PathBuf>,
    extensions: HashMap<String, usize>,
    total_bytes: usize,
    start_time: Instant,
}


impl Default for StatsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl StatsCollector {
    /// Create a new statistics collector
    pub fn new() -> Self {
        Self {
            files_processed: 0,
            directories_processed: 0,
            binary_files: 0,
            text_files: 0,
            unreadable_files: 0,
            skipped_files: 0,
            skipped_directories: 0,
            skipped_large_files: 0,
            gitignored_files: 0,
            gitignored_directories: 0,
            gitignore_files: Vec::new(),
            extensions: HashMap::new(),
            total_bytes: 0,
            start_time: Instant::now(),
        }
    }

    /// Record a processed text file
    pub fn record_text_file(&mut self, path: &std::path::Path, size: usize) {
        self.files_processed += 1;
        self.text_files += 1;
        self.total_bytes += size;

        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            *self.extensions.entry(ext_str).or_insert(0) += 1;
        }
    }

    /// Record a processed binary file
    pub fn record_binary_file(&mut self, path: &std::path::Path) {
        self.files_processed += 1;
        self.binary_files += 1;

        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            *self.extensions.entry(ext_str).or_insert(0) += 1;
        }
    }

    /// Record an unreadable file
    pub fn record_unreadable_file(&mut self) {
        self.files_processed += 1;
        self.unreadable_files += 1;
    }

    /// Record a processed directory
    pub fn record_directory(&mut self) {
        self.directories_processed += 1;
    }

    /// Record a skipped file
    pub fn record_skipped_file(&mut self) {
        self.skipped_files += 1;
    }

    /// Record a skipped directory
    pub fn record_skipped_directory(&mut self) {
        self.skipped_directories += 1;
    }

    /// Record a gitignored file
    pub fn record_gitignored_file(&mut self) {
        self.gitignored_files += 1;
    }

    /// Record a gitignored directory
    pub fn record_gitignored_directory(&mut self) {
        self.gitignored_directories += 1;
    }

    /// Record a large file that was skipped
    pub fn record_skipped_large_file(&mut self) {
        self.skipped_large_files += 1;
    }

    /// Set gitignore files being used
    pub fn set_gitignore_active(&mut self, gitignore_files: Vec<PathBuf>) {
        self.gitignore_files = gitignore_files;
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Format statistics for display
    pub fn format_stats(&self) -> String {
        let elapsed = self.elapsed();

        let mut output = Vec::new();

        // Summary line
        output.push(format!(
            "Processed {} files and {} directories in {:.2}s",
            self.files_processed,
            self.directories_processed,
            elapsed.as_secs_f64()
        ));

        // Gitignore info
        if !self.gitignore_files.is_empty() {
            let gitignore_names: Vec<String> = self
                .gitignore_files
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            output.push(format!("Using .gitignore: {}", gitignore_names.join(", ")));
        }

        // File type breakdown
        if self.files_processed > 0 {
            output.push(format!(
                "Files: {} text, {} binary, {} unreadable",
                self.text_files, self.binary_files, self.unreadable_files
            ));
        }

        // Skipped items
        let total_skipped_files = self.skipped_files + self.binary_files + self.gitignored_files + self.skipped_large_files;
        let total_skipped_dirs = self.skipped_directories + self.gitignored_directories;

        if total_skipped_files > 0 || total_skipped_dirs > 0 {
            let mut skip_reasons = Vec::new();

            if self.skipped_files + self.binary_files > 0 {
                skip_reasons.push(format!(
                    "{} hidden/binary",
                    self.skipped_files + self.binary_files
                ));
            }
            if self.skipped_large_files > 0 {
                skip_reasons.push(format!(
                    "{} too large",
                    self.skipped_large_files
                ));
            }
            if self.gitignored_files + self.gitignored_directories > 0 {
                skip_reasons.push(format!(
                    "{} gitignored",
                    self.gitignored_files + self.gitignored_directories
                ));
            }

            output.push(format!(
                "Skipped: {} files, {} directories ({})",
                total_skipped_files,
                total_skipped_dirs,
                skip_reasons.join(", ")
            ));
        }

        // Top extensions
        if !self.extensions.is_empty() {
            let mut extensions: Vec<_> = self.extensions.iter().collect();
            extensions.sort_by(|a, b| b.1.cmp(a.1));

            let top_exts: Vec<String> = extensions
                .iter()
                .take(10)
                .map(|(ext, count)| format!(".{} ({})", ext, count))
                .collect();

            if !top_exts.is_empty() {
                output.push(format!("Top extensions: {}", top_exts.join(", ")));
            }
        }

        // Processing speed
        if elapsed.as_secs_f64() > 0.0 {
            let files_per_sec = self.files_processed as f64 / elapsed.as_secs_f64();
            let mb_per_sec = (self.total_bytes as f64 / 1024.0 / 1024.0) / elapsed.as_secs_f64();
            output.push(format!(
                "Speed: {:.0} files/sec, {:.2} MB/sec",
                files_per_sec, mb_per_sec
            ));
        }

        output.join("\n")
    }
}
