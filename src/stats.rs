use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Statistics collector for tracking processing metrics
#[derive(Clone)]
pub struct StatsCollector {
    inner: Arc<Mutex<Stats>>,
    start_time: Instant,
}

/// Internal statistics data
struct Stats {
    files_processed: usize,
    directories_processed: usize,
    binary_files: usize,
    text_files: usize,
    unreadable_files: usize,
    extensions: HashMap<String, usize>,
    total_bytes: usize,
}

impl StatsCollector {
    /// Create a new statistics collector
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Stats {
                files_processed: 0,
                directories_processed: 0,
                binary_files: 0,
                text_files: 0,
                unreadable_files: 0,
                extensions: HashMap::new(),
                total_bytes: 0,
            })),
            start_time: Instant::now(),
        }
    }

    /// Record a processed text file
    pub fn record_text_file(&self, path: &std::path::Path, size: usize) {
        let mut stats = self.inner.lock().unwrap();
        stats.files_processed += 1;
        stats.text_files += 1;
        stats.total_bytes += size;
        
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            *stats.extensions.entry(ext_str).or_insert(0) += 1;
        }
    }

    /// Record a processed binary file
    pub fn record_binary_file(&self, path: &std::path::Path) {
        let mut stats = self.inner.lock().unwrap();
        stats.files_processed += 1;
        stats.binary_files += 1;
        
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            *stats.extensions.entry(ext_str).or_insert(0) += 1;
        }
    }

    /// Record an unreadable file
    pub fn record_unreadable_file(&self) {
        let mut stats = self.inner.lock().unwrap();
        stats.files_processed += 1;
        stats.unreadable_files += 1;
    }

    /// Record a processed directory
    pub fn record_directory(&self) {
        let mut stats = self.inner.lock().unwrap();
        stats.directories_processed += 1;
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Format statistics for display
    pub fn format_stats(&self) -> String {
        let stats = self.inner.lock().unwrap();
        let elapsed = self.elapsed();
        
        let mut output = Vec::new();
        
        // Summary line
        output.push(format!(
            "Processed {} files and {} directories in {:.2}s",
            stats.files_processed,
            stats.directories_processed,
            elapsed.as_secs_f64()
        ));
        
        // File type breakdown
        if stats.files_processed > 0 {
            output.push(format!(
                "Files: {} text, {} binary, {} unreadable",
                stats.text_files,
                stats.binary_files,
                stats.unreadable_files
            ));
        }
        
        // Top extensions
        if !stats.extensions.is_empty() {
            let mut extensions: Vec<_> = stats.extensions.iter().collect();
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
            let files_per_sec = stats.files_processed as f64 / elapsed.as_secs_f64();
            let mb_per_sec = (stats.total_bytes as f64 / 1024.0 / 1024.0) / elapsed.as_secs_f64();
            output.push(format!(
                "Speed: {:.0} files/sec, {:.2} MB/sec",
                files_per_sec,
                mb_per_sec
            ));
        }
        
        output.join("\n")
    }
}