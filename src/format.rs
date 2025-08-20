/// Utilities for formatting byte sizes
pub struct ByteFormatter;

impl ByteFormatter {
    /// Format bytes into human-readable string with appropriate unit
    pub fn format(bytes: usize) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        const THRESHOLD: f64 = 1024.0;
        
        if bytes == 0 {
            return "0 B".to_string();
        }
        
        let mut size = bytes as f64;
        let mut unit_index = 0;
        
        while size >= THRESHOLD && unit_index < UNITS.len() - 1 {
            size /= THRESHOLD;
            unit_index += 1;
        }
        
        // Format with appropriate precision
        if size.fract() == 0.0 {
            format!("{:.0} {}", size, UNITS[unit_index])
        } else if size < 10.0 {
            format!("{:.2} {}", size, UNITS[unit_index])
        } else if size < 100.0 {
            format!("{:.1} {}", size, UNITS[unit_index])
        } else {
            format!("{:.0} {}", size, UNITS[unit_index])
        }
    }
    
    /// Format bytes into a specific unit without decimal places
    pub fn format_as_unit(bytes: usize) -> String {
        const GB: usize = 1024 * 1024 * 1024;
        const MB: usize = 1024 * 1024;
        const KB: usize = 1024;
        
        if bytes >= GB && bytes % GB == 0 {
            format!("{}GB", bytes / GB)
        } else if bytes >= MB && bytes % MB == 0 {
            format!("{}MB", bytes / MB)
        } else if bytes >= KB && bytes % KB == 0 {
            format!("{}KB", bytes / KB)
        } else {
            format!("{} bytes", bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_bytes() {
        assert_eq!(ByteFormatter::format(0), "0 B");
        assert_eq!(ByteFormatter::format(512), "512 B");
        assert_eq!(ByteFormatter::format(1024), "1 KB");
        assert_eq!(ByteFormatter::format(1536), "1.50 KB");
        assert_eq!(ByteFormatter::format(1024 * 1024), "1 MB");
        assert_eq!(ByteFormatter::format(5 * 1024 * 1024), "5 MB");
        assert_eq!(ByteFormatter::format(1024 * 1024 * 1024), "1 GB");
    }
    
    #[test]
    fn test_format_as_unit() {
        assert_eq!(ByteFormatter::format_as_unit(5 * 1024 * 1024), "5MB");
        assert_eq!(ByteFormatter::format_as_unit(50 * 1024 * 1024), "50MB");
        assert_eq!(ByteFormatter::format_as_unit(1024 * 1024 * 1024), "1GB");
        assert_eq!(ByteFormatter::format_as_unit(5 * 1024 * 1024 * 1024), "5GB");
    }
}