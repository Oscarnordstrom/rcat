use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};

use crate::config::Config;
use crate::file_processor::FileProcessor;
use crate::format::ByteFormatter;
use crate::gitignore::GitignoreManager;
use crate::stats::StatsCollector;
use crate::thread_pool::{SharedWorkQueue, get_thread_count};

/// Size tracking for enforcing limits
struct SizeTracker {
    current: Arc<Mutex<usize>>,
}

impl SizeTracker {
    fn new() -> Self {
        Self {
            current: Arc::new(Mutex::new(0)),
        }
    }

    fn try_add(&self, size: usize) -> bool {
        let mut current = self.current.lock().unwrap();
        if *current + size <= Config::MAX_SIZE {
            *current += size;
            true
        } else {
            false
        }
    }

    fn is_at_limit(&self) -> bool {
        let current = self.current.lock().unwrap();
        *current >= Config::MAX_SIZE
    }

    fn clone(&self) -> Self {
        Self {
            current: Arc::clone(&self.current),
        }
    }
}

/// Options for walking the directory tree
#[derive(Clone)]
pub struct WalkOptions {
    pub include_all: bool,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            include_all: false,
        }
    }
}

/// Result of walking a directory tree
pub struct WalkResult {
    pub content: String,
    pub stats: StatsCollector,
}

/// Main entry point for walking directory tree and collecting contents
pub fn walk_and_collect(paths: &[PathBuf], options: WalkOptions) -> io::Result<WalkResult> {
    let walker = DirectoryWalker::new(paths, options);
    walker.walk()
}

/// Handles parallel directory traversal
struct DirectoryWalker {
    work_queue: SharedWorkQueue,
    size_tracker: SizeTracker,
    stats: StatsCollector,
    options: WalkOptions,
    gitignore_managers: Vec<GitignoreManager>,
}

impl DirectoryWalker {
    /// Create a new directory walker
    fn new(paths: &[PathBuf], options: WalkOptions) -> Self {
        let work_queue = SharedWorkQueue::new();
        let mut gitignore_managers = Vec::new();
        let stats = StatsCollector::new();
        
        // Initialize work queue and gitignore managers for each path
        for path in paths {
            work_queue.push_initial(path.clone());
            
            let gitignore = GitignoreManager::new(path);
            
            // Record if gitignore is active
            if gitignore.has_active_gitignores() {
                let gitignore_files = gitignore.active_gitignores();
                stats.set_gitignore_active(gitignore_files);
            }
            
            gitignore_managers.push(gitignore);
        }
        
        Self {
            work_queue,
            size_tracker: SizeTracker::new(),
            stats,
            options,
            gitignore_managers,
        }
    }

    /// Walk the directory tree and collect all contents
    fn walk(self) -> io::Result<WalkResult> {
        let (sender, receiver) = mpsc::channel();
        
        // Spawn worker threads
        let workers = self.spawn_workers(sender);
        
        // Collect results
        let contents = self.collect_results(receiver);
        
        // Wait for workers to finish
        for worker in workers {
            worker.join().ok();
        }
        
        Ok(WalkResult {
            content: contents.join("\n"),
            stats: self.stats,
        })
    }

    /// Spawn worker threads
    fn spawn_workers(&self, result_sender: Sender<String>) -> Vec<JoinHandle<()>> {
        let num_threads = get_thread_count();
        
        (0..num_threads)
            .map(|_| {
                let queue = self.work_queue.clone();
                let sender = result_sender.clone();
                let tracker = self.size_tracker.clone();
                let stats = self.stats.clone();
                let options = self.options.clone();
                let gitignore_managers = self.gitignore_managers.clone();
                
                thread::spawn(move || {
                    Worker::new(queue, sender, tracker, stats, options, gitignore_managers).run();
                })
            })
            .collect()
    }

    /// Collect results from workers
    fn collect_results(&self, receiver: mpsc::Receiver<String>) -> Vec<String> {
        let mut contents = Vec::new();
        let mut total_size = 0;
        
        while let Ok(content) = receiver.recv() {
            let content_size = content.len();
            
            if total_size + content_size > Config::MAX_SIZE {
                contents.push(format!(
                    "\n--- TRUNCATED: Size limit of {} exceeded ---",
                    ByteFormatter::format_as_unit(Config::MAX_SIZE)
                ));
                self.work_queue.shutdown();
                break;
            }
            
            total_size += content_size;
            contents.push(content);
        }
        
        contents
    }
}

/// Worker thread for processing paths
struct Worker {
    work_queue: SharedWorkQueue,
    result_sender: Sender<String>,
    size_tracker: SizeTracker,
    stats: StatsCollector,
    options: WalkOptions,
    gitignore_managers: Vec<GitignoreManager>,
}

impl Worker {
    fn new(
        work_queue: SharedWorkQueue,
        result_sender: Sender<String>,
        size_tracker: SizeTracker,
        stats: StatsCollector,
        options: WalkOptions,
        gitignore_managers: Vec<GitignoreManager>,
    ) -> Self {
        Self {
            work_queue,
            result_sender,
            size_tracker,
            stats,
            options,
            gitignore_managers,
        }
    }

    /// Main worker loop
    fn run(self) {
        while let Some(path) = self.work_queue.pop() {
            if self.size_tracker.is_at_limit() {
                self.work_queue.complete_task();
                return;
            }
            
            self.process_path(&path);
            self.work_queue.complete_task();
        }
    }

    /// Process a single path
    fn process_path(&self, path: &Path) {
        // Check gitignore first (unless --all is specified)
        if !self.options.include_all {
            for gitignore in &self.gitignore_managers {
                if gitignore.should_ignore(path) {
                    if path.is_file() {
                        self.stats.record_gitignored_file();
                    } else if path.is_dir() {
                        self.stats.record_gitignored_directory();
                    }
                    return;
                }
            }
        }
        
        if path.is_file() {
            // Skip hidden files (starting with '.') unless --all is specified
            if !self.options.include_all {
                if let Some(file_name) = path.file_name() {
                    if let Some(name_str) = file_name.to_str() {
                        if name_str.starts_with('.') {
                            self.stats.record_skipped_file();
                            return;
                        }
                    }
                }
            }
            self.process_file(path);
        } else if path.is_dir() {
            // Skip hidden directories (starting with '.') unless --all is specified
            if !self.options.include_all {
                if let Some(dir_name) = path.file_name() {
                    if let Some(name_str) = dir_name.to_str() {
                        if name_str.starts_with('.') {
                            self.stats.record_skipped_directory();
                            return;
                        }
                    }
                }
            }
            self.process_directory(path);
        }
    }

    /// Process a file
    fn process_file(&self, path: &Path) {
        use crate::file_processor::FileContent;
        
        let content = FileProcessor::process(path);
        
        match &content {
            FileContent::Text(_) => {
                if let Some(formatted) = FileProcessor::format_content(path, content) {
                    let size = formatted.len();
                    if self.size_tracker.try_add(size) {
                        self.stats.record_text_file(path, size);
                        let _ = self.result_sender.send(formatted);
                    } else {
                        // Size limit reached, trigger shutdown
                        self.work_queue.shutdown();
                    }
                }
            }
            FileContent::Binary => {
                self.stats.record_binary_file(path);
                // Skip binary files unless --all is specified
                if self.options.include_all {
                    if let Some(formatted) = FileProcessor::format_content(path, content) {
                        if self.size_tracker.try_add(formatted.len()) {
                            let _ = self.result_sender.send(formatted);
                        } else {
                            self.work_queue.shutdown();
                        }
                    }
                }
            }
            FileContent::Unreadable => {
                self.stats.record_unreadable_file();
            }
        }
    }

    /// Process a directory
    fn process_directory(&self, path: &Path) {
        // Check if we should stop before processing directory entries
        if self.size_tracker.is_at_limit() || self.work_queue.is_shutdown() {
            return;
        }
        
        // Record this directory in statistics
        self.stats.record_directory();
        
        // Check for .gitignore in this directory for all managers
        for gitignore in &self.gitignore_managers {
            gitignore.check_directory(path);
            
            // Update stats if we found a new gitignore
            if gitignore.has_active_gitignores() {
                let gitignore_files = gitignore.active_gitignores();
                self.stats.set_gitignore_active(gitignore_files);
            }
        }
        
        if let Ok(entries) = fs::read_dir(path) {
            let paths: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .collect();
            
            self.work_queue.extend(paths);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn setup_test_dir(name: &str) -> PathBuf {
        let dir = PathBuf::from(format!("test_{}", name));
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir(&dir).unwrap();
        dir
    }

    fn cleanup_test_dir(dir: &Path) {
        if dir.exists() {
            fs::remove_dir_all(dir).unwrap();
        }
    }

    #[test]
    fn test_walk_and_collect_single_file() {
        let dir = setup_test_dir("single");
        let file_path = dir.join("test.txt");
        fs::write(&file_path, "test content").unwrap();
        
        let result = walk_and_collect(&[dir.clone()], WalkOptions::default()).unwrap();
        
        assert!(result.content.contains("test content"));
        assert!(result.content.contains("test.txt"));
        
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_walk_and_collect_binary_file() {
        let dir = setup_test_dir("walk_binary");
        let file_path = dir.join("binary.dat");
        
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(&[0u8; 100]).unwrap();
        
        // Binary files should be skipped by default
        let result = walk_and_collect(&[dir.clone()], WalkOptions::default()).unwrap();
        assert!(!result.content.contains("<BINARY_FILE>"));
        
        // But included with include_all option
        let result = walk_and_collect(&[dir.clone()], WalkOptions { include_all: true }).unwrap();
        assert!(result.content.contains("<BINARY_FILE>"));
        assert!(result.content.contains("binary.dat"));
        
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_walk_and_collect_nested_directories() {
        let dir = setup_test_dir("nested");
        
        fs::create_dir_all(dir.join("subdir1/subdir2")).unwrap();
        fs::write(dir.join("root.txt"), "root file").unwrap();
        fs::write(dir.join("subdir1/level1.txt"), "level 1").unwrap();
        fs::write(dir.join("subdir1/subdir2/level2.txt"), "level 2").unwrap();
        
        let result = walk_and_collect(&[dir.clone()], WalkOptions::default()).unwrap();
        
        assert!(result.content.contains("root file"));
        assert!(result.content.contains("level 1"));
        assert!(result.content.contains("level 2"));
        
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_walk_and_collect_empty_directory() {
        let dir = setup_test_dir("empty");
        
        let result = walk_and_collect(&[dir.clone()], WalkOptions::default()).unwrap();
        
        assert_eq!(result.content, "");
        
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_size_limit_enforcement() {
        let dir = setup_test_dir("size_limit");
        
        // Create files that together exceed MAX_SIZE
        for i in 0..200 {
            let content = "x".repeat(30_000); // 30KB per file
            fs::write(dir.join(format!("file_{}.txt", i)), content).unwrap();
        }
        
        let result = walk_and_collect(&[dir.clone()], WalkOptions::default()).unwrap();
        
        // Result should be under MAX_SIZE plus some overhead
        assert!(result.content.len() <= Config::MAX_SIZE + 1000);
        
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_skip_hidden_files_and_directories() {
        let dir = setup_test_dir("hidden");
        
        // Create hidden files and directories
        fs::write(dir.join(".env"), "secret=value").unwrap();
        fs::write(dir.join(".hidden_file"), "hidden content").unwrap();
        fs::write(dir.join("visible.txt"), "visible content").unwrap();
        
        fs::create_dir(dir.join(".git")).unwrap();
        fs::write(dir.join(".git/config"), "git config").unwrap();
        
        // Default: skip hidden files and directories
        let result = walk_and_collect(&[dir.clone()], WalkOptions::default()).unwrap();
        assert!(!result.content.contains("secret=value"));
        assert!(!result.content.contains("hidden content"));
        assert!(!result.content.contains("git config"));
        assert!(result.content.contains("visible content"));
        
        // With include_all: include hidden files and directories
        let result = walk_and_collect(&[dir.clone()], WalkOptions { include_all: true }).unwrap();
        assert!(result.content.contains("secret=value"));
        assert!(result.content.contains("hidden content"));
        assert!(result.content.contains("git config"));
        assert!(result.content.contains("visible content"));
        
        cleanup_test_dir(&dir);
    }
}