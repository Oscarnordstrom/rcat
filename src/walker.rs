use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::file_processor::FileProcessor;
use crate::format::ByteFormatter;
use crate::gitignore::GitignoreManager;
use crate::stats::StatsCollector;

/// Options for walking the directory tree
#[derive(Clone)]
pub struct WalkOptions {
    pub include_all: bool,
    pub max_size: usize,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            include_all: false,
            max_size: Config::DEFAULT_MAX_SIZE,
        }
    }
}

/// Result of walking a directory tree
pub struct WalkResult {
    pub content: String,
    pub stats: StatsCollector,
    pub truncated: bool,
}

/// Main entry point for walking directory tree and collecting contents
pub fn walk_and_collect(paths: &[PathBuf], options: WalkOptions) -> io::Result<WalkResult> {
    let mut walker = DirectoryWalker::new(options);
    
    for path in paths {
        walker.add_root(path);
    }
    
    walker.walk()
}

/// Handles directory traversal
struct DirectoryWalker {
    contents: Vec<String>,
    total_size: usize,
    truncated: bool,
    stats: StatsCollector,
    options: WalkOptions,
    gitignore_managers: Vec<GitignoreManager>,
    root_paths: Vec<PathBuf>,
}

impl DirectoryWalker {
    /// Create a new directory walker
    fn new(options: WalkOptions) -> Self {
        Self {
            contents: Vec::new(),
            total_size: 0,
            truncated: false,
            stats: StatsCollector::new(),
            options,
            gitignore_managers: Vec::new(),
            root_paths: Vec::new(),
        }
    }
    
    /// Add a root path to process
    fn add_root(&mut self, path: &Path) {
        self.root_paths.push(path.to_path_buf());
        
        let gitignore = GitignoreManager::new(path);
        
        // Record if gitignore is active
        if gitignore.has_active_gitignores() {
            let gitignore_files = gitignore.active_gitignores();
            self.stats.set_gitignore_active(gitignore_files);
        }
        
        self.gitignore_managers.push(gitignore);
    }
    
    /// Walk the directory tree and collect all contents
    fn walk(mut self) -> io::Result<WalkResult> {
        // Process each root path
        for path in self.root_paths.clone() {
            if self.truncated {
                break;
            }
            self.process_path(&path)?;
        }
        
        Ok(WalkResult {
            content: self.contents.join("\n"),
            stats: self.stats,
            truncated: self.truncated,
        })
    }
    
    /// Process a single path (file or directory)
    fn process_path(&mut self, path: &Path) -> io::Result<()> {
        if self.truncated {
            return Ok(());
        }
        
        // Check gitignore first (unless --all is specified)
        if !self.options.include_all {
            for gitignore in &self.gitignore_managers {
                if gitignore.should_ignore(path) {
                    if path.is_file() {
                        self.stats.record_gitignored_file();
                    } else if path.is_dir() {
                        self.stats.record_gitignored_directory();
                    }
                    return Ok(());
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
                            return Ok(());
                        }
                    }
                }
            }
            self.process_file(path)?;
        } else if path.is_dir() {
            // Skip hidden directories (starting with '.') unless --all is specified
            if !self.options.include_all {
                if let Some(dir_name) = path.file_name() {
                    if let Some(name_str) = dir_name.to_str() {
                        if name_str.starts_with('.') {
                            self.stats.record_skipped_directory();
                            return Ok(());
                        }
                    }
                }
            }
            self.process_directory(path)?;
        }
        
        Ok(())
    }
    
    /// Process a file
    fn process_file(&mut self, path: &Path) -> io::Result<()> {
        use crate::file_processor::FileContent;
        
        let content = FileProcessor::process(path);
        
        match &content {
            FileContent::Text(_) => {
                if let Some(formatted) = FileProcessor::format_content(path, content) {
                    let size = formatted.len();
                    
                    // Check if adding this would exceed the limit
                    if self.total_size + size > self.options.max_size {
                        self.contents.push(format!(
                            "\n--- TRUNCATED: Size limit of {} reached ---\n--- {} collected, {} would exceed limit ---",
                            ByteFormatter::format_as_unit(self.options.max_size),
                            ByteFormatter::format(self.total_size),
                            ByteFormatter::format(self.total_size + size)
                        ));
                        self.truncated = true;
                        return Ok(());
                    }
                    
                    self.total_size += size;
                    self.stats.record_text_file(path, size);
                    self.contents.push(formatted);
                }
            }
            FileContent::Binary => {
                self.stats.record_binary_file(path);
                // Skip binary files unless --all is specified
                if self.options.include_all {
                    if let Some(formatted) = FileProcessor::format_content(path, content) {
                        let size = formatted.len();
                        
                        if self.total_size + size > self.options.max_size {
                            self.contents.push(format!(
                                "\n--- TRUNCATED: Size limit of {} reached ---\n--- {} collected, {} would exceed limit ---",
                                ByteFormatter::format_as_unit(self.options.max_size),
                                ByteFormatter::format(self.total_size),
                                ByteFormatter::format(self.total_size + size)
                            ));
                            self.truncated = true;
                            return Ok(());
                        }
                        
                        self.total_size += size;
                        self.contents.push(formatted);
                    }
                }
            }
            FileContent::Unreadable => {
                self.stats.record_unreadable_file();
            }
        }
        
        Ok(())
    }
    
    /// Process a directory
    fn process_directory(&mut self, path: &Path) -> io::Result<()> {
        if self.truncated {
            return Ok(());
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
        
        // Read directory entries and sort them for deterministic ordering
        let mut entries: Vec<PathBuf> = fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        
        // Sort entries for consistent ordering
        entries.sort();
        
        // Process each entry
        for entry in entries {
            if self.truncated {
                break;
            }
            self.process_path(&entry)?;
        }
        
        Ok(())
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
        let result = walk_and_collect(&[dir.clone()], WalkOptions { include_all: true, max_size: Config::DEFAULT_MAX_SIZE }).unwrap();
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
    #[ignore] // TODO: Fix after removing multithreading
    fn test_size_limit_enforcement() {
        let dir = setup_test_dir("size_limit");
        
        // Create files that together exceed DEFAULT_MAX_SIZE
        for i in 0..200 {
            let content = "x".repeat(30_000); // 30KB per file
            fs::write(dir.join(format!("file_{}.txt", i)), content).unwrap();
        }
        
        let result = walk_and_collect(&[dir.clone()], WalkOptions::default()).unwrap();
        
        // Result should be under DEFAULT_MAX_SIZE plus some overhead
        assert!(result.content.len() <= Config::DEFAULT_MAX_SIZE + 10000);
        
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
        let result = walk_and_collect(&[dir.clone()], WalkOptions { include_all: true, max_size: Config::DEFAULT_MAX_SIZE }).unwrap();
        assert!(result.content.contains("secret=value"));
        assert!(result.content.contains("hidden content"));
        assert!(result.content.contains("git config"));
        assert!(result.content.contains("visible content"));
        
        cleanup_test_dir(&dir);
    }
}