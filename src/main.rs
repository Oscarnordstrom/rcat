use std::env;
use std::path::PathBuf;
use std::process;

use rcat::{walk_and_collect, WalkResult, format::ByteFormatter, Config, WalkOptions};

mod clipboard;

/// Application metadata
struct AppInfo;

impl AppInfo {
    const NAME: &'static str = "rcat";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const DESCRIPTION: &'static str = "Recursively concatenate files and copy to clipboard";
}

/// Command-line arguments
struct Args {
    paths: Vec<PathBuf>,
    include_all: bool,
}

impl Args {
    /// Parse command-line arguments
    fn parse() -> Result<Self, ArgsError> {
        let args: Vec<String> = env::args().collect();
        
        if args.len() < 2 {
            return Err(ArgsError::InvalidCount);
        }
        
        let mut include_all = false;
        let mut paths = Vec::new();
        
        for arg in args.iter().skip(1) {
            match arg.as_str() {
                "--help" | "-h" => return Err(ArgsError::HelpRequested),
                "--all" | "-a" => include_all = true,
                path_str => {
                    let path = PathBuf::from(path_str);
                    if !path.exists() {
                        return Err(ArgsError::PathNotFound(path));
                    }
                    paths.push(path);
                }
            }
        }
        
        if paths.is_empty() {
            return Err(ArgsError::InvalidCount);
        }
        
        Ok(Args { paths, include_all })
    }
}

/// Argument parsing errors
enum ArgsError {
    InvalidCount,
    HelpRequested,
    PathNotFound(PathBuf),
}

/// Print help message
fn print_help(program_name: &str) {
    println!("{} v{}", AppInfo::NAME, AppInfo::VERSION);
    println!("{}", AppInfo::DESCRIPTION);
    println!();
    println!("Usage: {} [OPTIONS] <path>...", program_name);
    println!();
    println!("Options:");
    println!("  --all, -a     Include hidden directories and binary files");
    println!("  --help, -h    Show this help message");
    println!();
    println!("Description:");
    println!("  Recursively walks through directories, concatenates all file contents,");
    println!("  and copies the result to the system clipboard.");
    println!();
    println!("  You can specify multiple paths to process them all together.");
    println!();
    println!("  By default, hidden directories (starting with '.') and binary files");
    println!("  are skipped. Use --all to include them.");
    println!();
    println!("  The total size is limited to {}.",
             ByteFormatter::format_as_unit(Config::MAX_SIZE));
}

/// Print error message
fn print_error(program_name: &str, error: ArgsError) {
    match error {
        ArgsError::InvalidCount => {
            eprintln!("Usage: {} [OPTIONS] <path>...", program_name);
            eprintln!("{}", AppInfo::DESCRIPTION);
            eprintln!("Try '{} --help' for more information", program_name);
        }
        ArgsError::PathNotFound(path) => {
            eprintln!("Error: Path '{}' does not exist", path.display());
        }
        ArgsError::HelpRequested => {
            print_help(program_name);
        }
    }
}

fn main() {
    let program_name = env::args().next().unwrap_or_else(|| AppInfo::NAME.to_string());
    
    let args = match Args::parse() {
        Ok(args) => args,
        Err(error) => {
            match error {
                ArgsError::HelpRequested => {
                    print_help(&program_name);
                    process::exit(0);
                }
                _ => {
                    print_error(&program_name, error);
                    process::exit(1);
                }
            }
        }
    };
    
    run(args);
}

/// Run the application
fn run(args: Args) {
    let options = WalkOptions {
        include_all: args.include_all,
    };
    
    match walk_and_collect(&args.paths, options) {
        Ok(result) => {
            handle_result(result);
        }
        Err(error) => {
            eprintln!("Error: Failed to process directories - {}", error);
            process::exit(1);
        }
    }
}

/// Handle the collected result
fn handle_result(result: WalkResult) {
    let size = result.content.len();
    
    if size == 0 {
        println!("No files found to copy");
        return;
    }
    
    match clipboard::copy_to_clipboard(&result.content) {
        Ok(_) => {
            println!("Successfully copied {} to clipboard", ByteFormatter::format(size));
            eprintln!("\n{}", result.stats.format_stats());
        }
        Err(error) => {
            eprintln!("Error: Failed to copy to clipboard - {}", error);
            process::exit(1);
        }
    }
}
