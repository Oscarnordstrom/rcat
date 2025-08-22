use std::io::{self, Write};
use std::process::{Command, Stdio};

/// Check if clipboard utility is available
pub fn validate_clipboard() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if !is_command_available("pbcopy") {
            return Err("pbcopy not found. This should be installed by default on macOS.".to_string());
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if !is_command_available("xclip") {
            return Err(
                "xclip not found. Install it with:\n  \
                Ubuntu/Debian: sudo apt install xclip\n  \
                Fedora: sudo dnf install xclip\n  \
                Arch: sudo pacman -S xclip".to_string()
            );
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        if !is_command_available("clip") {
            return Err("clip.exe not found. This should be installed by default on Windows.".to_string());
        }
    }
    
    Ok(())
}

/// Check if a command is available in PATH
fn is_command_available(cmd: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("where")
            .arg(cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

pub fn copy_to_clipboard(content: &str) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let mut child = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()?;
        
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(content.as_bytes())?;
        }
        
        child.wait()?;
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    {
        let mut child = Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .stdin(Stdio::piped())
            .spawn()?;
        
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(content.as_bytes())?;
        }
        
        child.wait()?;
        Ok(())
    }
    
    #[cfg(target_os = "windows")]
    {
        let mut child = Command::new("cmd")
            .args(&["/C", "clip"])
            .stdin(Stdio::piped())
            .spawn()?;
        
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(content.as_bytes())?;
        }
        
        child.wait()?;
        Ok(())
    }
}