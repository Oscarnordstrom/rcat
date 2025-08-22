/// Configuration constants for the application
pub struct Config;

impl Config {
    /// Default maximum size of content to copy to clipboard (5MB)
    pub const DEFAULT_MAX_SIZE: usize = 5 * 1024 * 1024;


    /// Buffer size for binary file detection
    pub const BINARY_CHECK_BUFFER_SIZE: usize = 8192;
}

/// Parse human-readable size string (e.g., "10MB", "1GB", "500KB")
pub fn parse_size(size_str: &str) -> Result<usize, String> {
    let size_str = size_str.trim().to_uppercase();
    
    // Find where the number ends and unit begins
    let (number_part, unit_part) = match size_str.find(|c: char| c.is_alphabetic()) {
        Some(pos) => (&size_str[..pos], &size_str[pos..]),
        None => (size_str.as_str(), "B"),
    };
    
    // Parse the number (trim any spaces)
    let number: f64 = number_part.trim().parse()
        .map_err(|_| format!("Invalid number: {}", number_part.trim()))?;
    
    if number < 0.0 {
        return Err("Size cannot be negative".to_string());
    }
    
    // Parse the unit
    let multiplier = match unit_part {
        "B" | "" => 1,
        "KB" | "K" => 1024,
        "MB" | "M" => 1024 * 1024,
        "GB" | "G" => 1024 * 1024 * 1024,
        _ => return Err(format!("Unknown unit: {}. Use B, KB, MB, or GB", unit_part)),
    };
    
    let size = (number * multiplier as f64) as usize;
    
    if size == 0 {
        return Err("Size must be greater than 0".to_string());
    }
    
    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("100").unwrap(), 100);
        assert_eq!(parse_size("100B").unwrap(), 100);
        assert_eq!(parse_size("1KB").unwrap(), 1024);
        assert_eq!(parse_size("1K").unwrap(), 1024);
        assert_eq!(parse_size("5MB").unwrap(), 5 * 1024 * 1024);
        assert_eq!(parse_size("5M").unwrap(), 5 * 1024 * 1024);
        assert_eq!(parse_size("1GB").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size("1G").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size("1.5MB").unwrap(), (1.5 * 1024.0 * 1024.0) as usize);
        assert_eq!(parse_size(" 10 MB ").unwrap(), 10 * 1024 * 1024);
        
        assert!(parse_size("invalid").is_err());
        assert!(parse_size("-5MB").is_err());
        assert!(parse_size("5TB").is_err());
    }
}
