use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Manages gitignore patterns hierarchically
#[derive(Clone)]
pub struct GitignoreManager {
    inner: Arc<Mutex<GitignoreManagerInner>>,
}

struct GitignoreManagerInner {
    // Map from directory path to its gitignore matcher
    matchers: HashMap<PathBuf, GitignoreMatcher>,
    // Track which gitignore files we've found
    active_gitignores: Vec<PathBuf>,
    // The root path we started from
    root_path: PathBuf,
}

impl GitignoreManager {
    /// Create a new gitignore manager starting from the given root path
    pub fn new(root_path: &Path) -> Self {
        let mut inner = GitignoreManagerInner {
            matchers: HashMap::new(),
            active_gitignores: Vec::new(),
            root_path: root_path.to_path_buf(),
        };

        // Check for .gitignore in the root directory
        let gitignore_path = root_path.join(".gitignore");
        if gitignore_path.exists()
            && let Ok(content) = fs::read_to_string(&gitignore_path)
        {
            let matcher = GitignoreMatcher::new(&content, root_path);
            inner.matchers.insert(root_path.to_path_buf(), matcher);
            inner.active_gitignores.push(gitignore_path);
        }

        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// Check and load gitignore for a directory if it exists
    pub fn check_directory(&self, dir_path: &Path) {
        let gitignore_path = dir_path.join(".gitignore");
        if gitignore_path.exists() {
            let mut inner = self.inner.lock().unwrap();

            // Only load if we haven't already
            if !inner.matchers.contains_key(dir_path)
                && let Ok(content) = fs::read_to_string(&gitignore_path)
            {
                let matcher = GitignoreMatcher::new(&content, dir_path);
                inner.matchers.insert(dir_path.to_path_buf(), matcher);
                inner.active_gitignores.push(gitignore_path);
            }
        }
    }

    /// Check if a path should be ignored based on all applicable gitignore files
    pub fn should_ignore(&self, path: &Path) -> bool {
        let inner = self.inner.lock().unwrap();

        // Check each gitignore from root down to the file's directory
        // We need to check all parent directories
        let mut current_path = inner.root_path.clone();

        // First check the root
        if let Some(matcher) = inner.matchers.get(&current_path)
            && matcher.should_ignore(path)
        {
            return true;
        }

        // Then check each subdirectory leading to the target
        if let Ok(relative) = path.strip_prefix(&inner.root_path) {
            for component in relative.components() {
                current_path.push(component);

                // Only check directories that have gitignore files
                if let Some(matcher) = inner.matchers.get(&current_path)
                    && matcher.should_ignore(path)
                {
                    return true;
                }
            }
        }

        false
    }

    /// Get the list of active gitignore files
    pub fn active_gitignores(&self) -> Vec<PathBuf> {
        self.inner.lock().unwrap().active_gitignores.clone()
    }

    /// Check if any gitignore files are active
    pub fn has_active_gitignores(&self) -> bool {
        !self.inner.lock().unwrap().active_gitignores.is_empty()
    }
}

/// A gitignore pattern matcher for a specific directory
struct GitignoreMatcher {
    patterns: Vec<Pattern>,
    base_path: PathBuf,
}

struct Pattern {
    pattern: String,
    is_negation: bool,
    is_directory_only: bool,
    is_absolute: bool,
}

impl GitignoreMatcher {
    /// Create a new gitignore matcher from content and base path
    fn new(content: &str, base_path: &Path) -> Self {
        let patterns = Self::parse_gitignore(content);
        Self {
            patterns,
            base_path: base_path.to_path_buf(),
        }
    }

    /// Check if a path should be ignored by this specific gitignore
    fn should_ignore(&self, path: &Path) -> bool {
        // Get the relative path from this gitignore's base
        let relative_path = match path.strip_prefix(&self.base_path) {
            Ok(rel) => rel,
            Err(_) => return false,
        };

        // Empty relative path means it's the base directory itself
        if relative_path.as_os_str().is_empty() {
            return false;
        }

        let path_str = relative_path.to_string_lossy();
        let is_dir = path.is_dir();

        let mut ignored = false;

        for pattern in &self.patterns {
            if pattern.is_directory_only && !is_dir {
                continue;
            }

            if self.matches_pattern(&path_str, &pattern.pattern, pattern.is_absolute) {
                ignored = !pattern.is_negation;
            }
        }

        ignored
    }

    /// Parse gitignore content into patterns
    fn parse_gitignore(content: &str) -> Vec<Pattern> {
        content
            .lines()
            .filter_map(|line| {
                let line = line.trim();

                // Skip empty lines and comments
                if line.is_empty() || line.starts_with('#') {
                    return None;
                }

                let is_negation = line.starts_with('!');
                let line = if is_negation { &line[1..] } else { line };

                let is_directory_only = line.ends_with('/');
                let line = if is_directory_only {
                    &line[..line.len() - 1]
                } else {
                    line
                };

                let is_absolute = line.starts_with('/');
                let pattern = if is_absolute {
                    line[1..].to_string()
                } else {
                    line.to_string()
                };

                Some(Pattern {
                    pattern,
                    is_negation,
                    is_directory_only,
                    is_absolute,
                })
            })
            .collect()
    }

    /// Check if a path matches a gitignore pattern
    fn matches_pattern(&self, path: &str, pattern: &str, is_absolute: bool) -> bool {
        // Handle simple cases first
        if pattern == "*" {
            return true;
        }

        // Convert pattern to a simple glob matcher
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if is_absolute {
            // Pattern must match from the beginning
            self.match_parts(&path_parts, &pattern_parts, 0)
        } else {
            // Pattern can match anywhere in the path
            // But if pattern contains /, it should match the full path structure
            if pattern.contains('/') {
                // Match against full path
                self.match_parts(&path_parts, &pattern_parts, 0)
            } else {
                // Match against any component
                for part in &path_parts {
                    if self.glob_match(part, pattern) {
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Match path parts against pattern parts
    fn match_parts(&self, path_parts: &[&str], pattern_parts: &[&str], start_idx: usize) -> bool {
        if pattern_parts.is_empty() {
            return true;
        }

        let mut path_idx = start_idx;
        let mut pattern_idx = 0;

        while pattern_idx < pattern_parts.len() && path_idx < path_parts.len() {
            let pattern_part = pattern_parts[pattern_idx];
            let path_part = path_parts[path_idx];

            if pattern_part == "**" {
                // Match zero or more directories
                if pattern_idx == pattern_parts.len() - 1 {
                    return true; // ** at end matches everything
                }

                // Try matching the rest of the pattern at different positions
                pattern_idx += 1;
                let next_pattern = pattern_parts[pattern_idx];

                // Try to find where the next pattern matches
                while path_idx < path_parts.len() {
                    if self.glob_match(path_parts[path_idx], next_pattern) {
                        // Found a match, continue matching from here
                        if self.match_parts(path_parts, &pattern_parts[pattern_idx..], path_idx) {
                            return true;
                        }
                    }
                    path_idx += 1;
                }
                return false;
            } else if self.glob_match(path_part, pattern_part) {
                path_idx += 1;
                pattern_idx += 1;
            } else {
                return false;
            }
        }

        pattern_idx == pattern_parts.len()
    }

    /// Simple glob matching for individual path components
    fn glob_match(&self, text: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if !pattern.contains('*') && !pattern.contains('?') {
            return text == pattern;
        }

        // Simple glob matching implementation
        let mut text_idx = 0;
        let mut pattern_idx = 0;
        let text_bytes = text.as_bytes();
        let pattern_bytes = pattern.as_bytes();

        let mut star_idx = None;
        let mut star_match = None;

        while text_idx < text_bytes.len() {
            if pattern_idx < pattern_bytes.len() {
                match pattern_bytes[pattern_idx] {
                    b'*' => {
                        star_idx = Some(pattern_idx);
                        star_match = Some(text_idx);
                        pattern_idx += 1;
                    }
                    b'?' => {
                        text_idx += 1;
                        pattern_idx += 1;
                    }
                    c if c == text_bytes[text_idx] => {
                        text_idx += 1;
                        pattern_idx += 1;
                    }
                    _ => {
                        if let (Some(s_idx), Some(s_match)) = (star_idx, star_match) {
                            pattern_idx = s_idx + 1;
                            star_match = Some(s_match + 1);
                            text_idx = s_match + 1;
                        } else {
                            return false;
                        }
                    }
                }
            } else if let (Some(s_idx), Some(s_match)) = (star_idx, star_match) {
                pattern_idx = s_idx + 1;
                star_match = Some(s_match + 1);
                text_idx = s_match + 1;
            } else {
                return false;
            }
        }

        // Check remaining pattern
        while pattern_idx < pattern_bytes.len() && pattern_bytes[pattern_idx] == b'*' {
            pattern_idx += 1;
        }

        pattern_idx == pattern_bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        let content = "";
        let matcher = GitignoreMatcher::new(content, Path::new("."));

        assert!(matcher.glob_match("test.txt", "*.txt"));
        assert!(matcher.glob_match("test.txt", "test.*"));
        assert!(matcher.glob_match("test.txt", "*.*"));
        assert!(matcher.glob_match("test.txt", "test.txt"));
        assert!(!matcher.glob_match("test.txt", "*.rs"));
        assert!(matcher.glob_match("a", "?"));
        assert!(!matcher.glob_match("ab", "?"));
    }

    #[test]
    fn test_parse_gitignore() {
        let content = "
# Comment
*.tmp
/build/
!important.tmp
node_modules/
**/*.log
        ";

        let patterns = GitignoreMatcher::parse_gitignore(content);
        assert_eq!(patterns.len(), 5);

        assert_eq!(patterns[0].pattern, "*.tmp");
        assert!(!patterns[0].is_negation);
        assert!(!patterns[0].is_directory_only);

        assert_eq!(patterns[1].pattern, "build");
        assert!(patterns[1].is_absolute);
        assert!(patterns[1].is_directory_only);

        assert_eq!(patterns[2].pattern, "important.tmp");
        assert!(patterns[2].is_negation);
    }
}
