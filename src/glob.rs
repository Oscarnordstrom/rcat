/// Utilities for glob pattern matching
pub struct GlobMatcher;

impl GlobMatcher {
    /// Simple glob matching for patterns supporting * and ? wildcards
    pub fn matches(text: &str, pattern: &str) -> bool {
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
    fn test_glob_matching() {
        assert!(GlobMatcher::matches("test.txt", "*.txt"));
        assert!(GlobMatcher::matches("test.txt", "test.*"));
        assert!(GlobMatcher::matches("test.txt", "*.*"));
        assert!(GlobMatcher::matches("test.txt", "test.txt"));
        assert!(!GlobMatcher::matches("test.txt", "*.rs"));
        assert!(GlobMatcher::matches("a", "?"));
        assert!(!GlobMatcher::matches("ab", "?"));
        assert!(GlobMatcher::matches("test_file", "test_*"));
        assert!(GlobMatcher::matches("anything", "*"));
    }
}