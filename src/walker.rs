use std::collections::{HashSet, VecDeque};
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
    pub max_file_size: usize,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            include_all: false,
            max_size: Config::DEFAULT_MAX_SIZE,
            max_file_size: Config::DEFAULT_MAX_FILE_SIZE,
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

/// Handles directory traversal using breadth-first search
struct DirectoryWalker {
    contents: Vec<String>,
    total_size: usize,
    truncated: bool,
    stats: StatsCollector,
    options: WalkOptions,
    gitignore_managers: Vec<GitignoreManager>,
    root_paths: Vec<PathBuf>,
    visited_paths: HashSet<PathBuf>,
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
            visited_paths: HashSet::new(),
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

    /// Walk the directory tree using breadth-first search
    fn walk(mut self) -> io::Result<WalkResult> {
        // Use a queue for BFS - process all files at each level before subdirectories
        let mut queue = VecDeque::new();

        // Add all root paths to the queue
        for path in self.root_paths.clone() {
            queue.push_back(path);
        }

        // Process queue in BFS order
        while let Some(path) = queue.pop_front() {
            if self.truncated {
                break;
            }

            // Process this path and collect subdirectories
            let subdirs = self.process_path_bfs(&path)?;

            // Add subdirectories to the end of the queue (BFS)
            for subdir in subdirs {
                queue.push_back(subdir);
            }
        }

        Ok(WalkResult {
            content: self.contents.join("\n"),
            stats: self.stats,
            truncated: self.truncated,
        })
    }

    /// Process a path and return any subdirectories to be queued
    fn process_path_bfs(&mut self, path: &Path) -> io::Result<Vec<PathBuf>> {
        if self.truncated {
            return Ok(Vec::new());
        }

        // Get canonical path to handle symlinks and deduplicate
        let canonical_path = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // If we can't canonicalize (e.g., broken symlink), skip this path
                return Ok(Vec::new());
            }
        };

        // Check if we've already visited this path
        if !self.visited_paths.insert(canonical_path.clone()) {
            // Path was already in the set, skip it
            return Ok(Vec::new());
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
                    return Ok(Vec::new());
                }
            }
        }

        if path.is_file() {
            // Skip hidden files (starting with '.') unless --all is specified
            if !self.options.include_all
                && let Some(file_name) = path.file_name()
                && let Some(name_str) = file_name.to_str()
                && name_str.starts_with('.')
            {
                self.stats.record_skipped_file();
                return Ok(Vec::new());
            }
            self.process_file(path)?;
            Ok(Vec::new())
        } else if path.is_dir() {
            // Skip hidden directories (starting with '.') unless --all is specified
            if !self.options.include_all
                && let Some(dir_name) = path.file_name()
                && let Some(name_str) = dir_name.to_str()
                && name_str.starts_with('.')
            {
                self.stats.record_skipped_directory();
                return Ok(Vec::new());
            }
            self.process_directory_bfs(path)
        } else {
            Ok(Vec::new())
        }
    }

    /// Process a directory in BFS manner - process files first, then return subdirs
    fn process_directory_bfs(&mut self, path: &Path) -> io::Result<Vec<PathBuf>> {
        if self.truncated {
            return Ok(Vec::new());
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

        // Read all entries
        let mut all_entries: Vec<PathBuf> = fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();

        // Sort for deterministic ordering
        all_entries.sort();

        // Separate files and directories
        let mut files = Vec::new();
        let mut subdirs = Vec::new();

        for entry in all_entries {
            // Check if we should skip this entry
            if !self.should_process(&entry) {
                continue;
            }

            if entry.is_file() {
                files.push(entry);
            } else if entry.is_dir() {
                subdirs.push(entry);
            }
        }

        // Process all files first (breadth-first within this directory)
        for file in files {
            if self.truncated {
                break;
            }
            self.process_file(&file)?;
        }

        // Return subdirectories to be processed later
        Ok(subdirs)
    }

    /// Check if a path should be processed
    fn should_process(&self, path: &Path) -> bool {
        // Check gitignore
        if !self.options.include_all {
            for gitignore in &self.gitignore_managers {
                if gitignore.should_ignore(path) {
                    if path.is_file() {
                        self.stats.record_gitignored_file();
                    } else if path.is_dir() {
                        self.stats.record_gitignored_directory();
                    }
                    return false;
                }
            }

            // Check for hidden files/directories
            if let Some(name) = path.file_name()
                && let Some(name_str) = name.to_str()
                && name_str.starts_with('.')
            {
                if path.is_file() {
                    self.stats.record_skipped_file();
                } else if path.is_dir() {
                    self.stats.record_skipped_directory();
                }
                return false;
            }
        }

        true
    }

    /// Process a file
    fn process_file(&mut self, path: &Path) -> io::Result<()> {
        use crate::file_processor::FileContent;

        // Check file size before processing
        if let Ok(metadata) = path.metadata() {
            let file_size = metadata.len() as usize;
            if file_size > self.options.max_file_size {
                self.stats.record_skipped_large_file();
                return Ok(());
            }
        }

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
                if self.options.include_all
                    && let Some(formatted) = FileProcessor::format_content(path, content)
                {
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
            FileContent::Unreadable => {
                self.stats.record_unreadable_file();
            }
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

        let result = walk_and_collect(std::slice::from_ref(&dir), WalkOptions::default()).unwrap();

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
        let result = walk_and_collect(std::slice::from_ref(&dir), WalkOptions::default()).unwrap();
        assert!(!result.content.contains("<BINARY_FILE>"));

        // But included with include_all option
        let result = walk_and_collect(
            std::slice::from_ref(&dir),
            WalkOptions {
                include_all: true,
                max_size: Config::DEFAULT_MAX_SIZE,
                max_file_size: Config::DEFAULT_MAX_FILE_SIZE,
            },
        )
        .unwrap();
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

        let result = walk_and_collect(std::slice::from_ref(&dir), WalkOptions::default()).unwrap();

        assert!(result.content.contains("root file"));
        assert!(result.content.contains("level 1"));
        assert!(result.content.contains("level 2"));

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_walk_and_collect_empty_directory() {
        let dir = setup_test_dir("empty");

        let result = walk_and_collect(std::slice::from_ref(&dir), WalkOptions::default()).unwrap();

        assert_eq!(result.content, "");

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_size_limit_enforcement() {
        let dir = setup_test_dir("size_limit");

        // Create files that together exceed DEFAULT_MAX_SIZE (5MB)
        // Files will be processed in alphabetical order
        for i in 0..20 {
            let content = "x".repeat(300_000); // 300KB per file = 6MB total
            fs::write(dir.join(format!("file_{:02}.txt", i)), content).unwrap();
        }

        let result = walk_and_collect(std::slice::from_ref(&dir), WalkOptions::default()).unwrap();

        // Result should be under DEFAULT_MAX_SIZE plus overhead for truncation message
        assert!(result.content.len() <= Config::DEFAULT_MAX_SIZE + 1000);

        // Should be truncated since we have 6MB of files and 5MB limit
        assert!(result.truncated, "Expected truncation");
        assert!(result.content.contains("TRUNCATED"));

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
        let result = walk_and_collect(std::slice::from_ref(&dir), WalkOptions::default()).unwrap();
        assert!(!result.content.contains("secret=value"));
        assert!(!result.content.contains("hidden content"));
        assert!(!result.content.contains("git config"));
        assert!(result.content.contains("visible content"));

        // With include_all: include hidden files and directories
        let result = walk_and_collect(
            std::slice::from_ref(&dir),
            WalkOptions {
                include_all: true,
                max_size: Config::DEFAULT_MAX_SIZE,
                max_file_size: Config::DEFAULT_MAX_FILE_SIZE,
            },
        )
        .unwrap();
        assert!(result.content.contains("secret=value"));
        assert!(result.content.contains("hidden content"));
        assert!(result.content.contains("git config"));
        assert!(result.content.contains("visible content"));

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_breadth_first_order() {
        let dir = setup_test_dir("bfs");

        // Create a structure to test BFS ordering
        // Root level files
        fs::write(dir.join("a_root.txt"), "root_a").unwrap();
        fs::write(dir.join("b_root.txt"), "root_b").unwrap();

        // Create subdirectories with files
        fs::create_dir(dir.join("dir1")).unwrap();
        fs::write(dir.join("dir1/a_level1.txt"), "level1_a").unwrap();
        fs::write(dir.join("dir1/b_level1.txt"), "level1_b").unwrap();

        fs::create_dir(dir.join("dir2")).unwrap();
        fs::write(dir.join("dir2/c_level1.txt"), "level1_c").unwrap();

        // Create nested subdirectory
        fs::create_dir(dir.join("dir1/subdir")).unwrap();
        fs::write(dir.join("dir1/subdir/deep.txt"), "deep_file").unwrap();

        let result = walk_and_collect(std::slice::from_ref(&dir), WalkOptions::default()).unwrap();

        // Find positions of each file in the output
        let pos_root_a = result.content.find("root_a").unwrap();
        let pos_root_b = result.content.find("root_b").unwrap();
        let pos_level1_a = result.content.find("level1_a").unwrap();
        let pos_level1_b = result.content.find("level1_b").unwrap();
        let pos_level1_c = result.content.find("level1_c").unwrap();
        let pos_deep = result.content.find("deep_file").unwrap();

        // BFS order: all root files should come before any level 1 files
        assert!(
            pos_root_a < pos_level1_a,
            "Root files should come before level 1"
        );
        assert!(
            pos_root_b < pos_level1_a,
            "Root files should come before level 1"
        );

        // All level 1 files should come before deep nested files
        assert!(
            pos_level1_a < pos_deep,
            "Level 1 should come before deeper levels"
        );
        assert!(
            pos_level1_b < pos_deep,
            "Level 1 should come before deeper levels"
        );
        assert!(
            pos_level1_c < pos_deep,
            "Level 1 should come before deeper levels"
        );

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_overlapping_paths_deduplication() {
        let dir = setup_test_dir("overlapping");

        // Create nested structure
        fs::create_dir(dir.join("subdir")).unwrap();
        fs::write(dir.join("file1.txt"), "content1").unwrap();
        fs::write(dir.join("subdir/file2.txt"), "content2").unwrap();

        // Pass both parent and child directory - should not duplicate file2.txt
        let result =
            walk_and_collect(&[dir.clone(), dir.join("subdir")], WalkOptions::default()).unwrap();

        // Each file content should appear exactly once
        let content1_count = result.content.matches("content1").count();
        let content2_count = result.content.matches("content2").count();

        assert_eq!(content1_count, 1, "file1.txt should appear exactly once");
        assert_eq!(
            content2_count, 1,
            "file2.txt should appear exactly once despite overlapping paths"
        );

        cleanup_test_dir(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn test_symlink_deduplication() {
        use std::os::unix::fs as unix_fs;

        let dir = setup_test_dir("symlinks");

        // Create a file and directory with content
        fs::write(dir.join("original.txt"), "original_content").unwrap();
        fs::create_dir(dir.join("original_dir")).unwrap();
        fs::write(dir.join("original_dir/nested.txt"), "nested_content").unwrap();

        // Create symlinks to the file and directory
        unix_fs::symlink(dir.join("original.txt"), dir.join("link_to_file.txt")).unwrap();
        unix_fs::symlink(dir.join("original_dir"), dir.join("link_to_dir")).unwrap();

        let result = walk_and_collect(std::slice::from_ref(&dir), WalkOptions::default()).unwrap();

        // Each content should appear exactly once despite symlinks
        let original_count = result.content.matches("original_content").count();
        let nested_count = result.content.matches("nested_content").count();

        assert_eq!(
            original_count, 1,
            "original.txt content should appear exactly once"
        );
        assert_eq!(
            nested_count, 1,
            "nested.txt content should appear exactly once"
        );

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_skip_large_files() {
        let dir = setup_test_dir("large_files");

        // Create a small file that should be included
        fs::write(dir.join("small.txt"), "small content").unwrap();

        // Create a large file that should be skipped (over 500KB)
        let large_content = "x".repeat(600_000); // 600KB
        fs::write(dir.join("large.txt"), &large_content).unwrap();

        // With default options (500KB limit)
        let result = walk_and_collect(std::slice::from_ref(&dir), WalkOptions::default()).unwrap();
        assert!(result.content.contains("small content"));
        assert!(!result.content.contains(&large_content));

        // With a higher file size limit
        let result = walk_and_collect(
            std::slice::from_ref(&dir),
            WalkOptions {
                include_all: false,
                max_size: Config::DEFAULT_MAX_SIZE,
                max_file_size: 1024 * 1024, // 1MB
            },
        )
        .unwrap();
        assert!(result.content.contains("small content"));
        assert!(result.content.contains(&large_content[..100])); // Check first 100 chars

        cleanup_test_dir(&dir);
    }
}
