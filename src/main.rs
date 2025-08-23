use std::env;
use std::path::PathBuf;
use std::process;

use rcat::{
    Config, WalkOptions, WalkResult, config::parse_size, format::ByteFormatter, walk_and_collect,
};

mod clipboard;

/// Application metadata
struct AppInfo;

impl AppInfo {
    const NAME: &'static str = "rcat";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const DESCRIPTION: &'static str = "Recursively concatenate files and copy to clipboard or output to stdout";
}

/// Command-line arguments
struct Args {
    paths: Vec<PathBuf>,
    include_all: bool,
    max_size: usize,
    max_file_size: usize,
    exclude_patterns: Vec<String>,
    stdout: bool,
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
        let mut max_size = Config::DEFAULT_MAX_SIZE;
        let mut max_file_size = Config::DEFAULT_MAX_FILE_SIZE;
        let mut exclude_patterns = Vec::new();
        let mut stdout = false;
        let mut skip_next = false;

        let mut iter = args.iter().skip(1).peekable();
        while let Some(arg) = iter.next() {
            if skip_next {
                skip_next = false;
                continue;
            }

            match arg.as_str() {
                "--help" | "-h" => return Err(ArgsError::HelpRequested),
                "--all" | "-a" => include_all = true,
                "--stdout" | "-o" => stdout = true,
                "--max-size" | "-m" => {
                    let size_str = iter.next().ok_or_else(|| {
                        ArgsError::InvalidSize("--max-size requires a value".to_string())
                    })?;
                    max_size = parse_size(size_str).map_err(ArgsError::InvalidSize)?;
                }
                "--max-file-size" | "-f" => {
                    let size_str = iter.next().ok_or_else(|| {
                        ArgsError::InvalidSize("--max-file-size requires a value".to_string())
                    })?;
                    max_file_size = parse_size(size_str).map_err(ArgsError::InvalidSize)?;
                }
                "--exclude" | "-e" => {
                    let pattern = iter.next().ok_or_else(|| {
                        ArgsError::InvalidSize("--exclude requires a pattern".to_string())
                    })?;
                    exclude_patterns.push(pattern.to_string());
                }
                path_str if path_str.starts_with('-') => {
                    return Err(ArgsError::UnknownOption(path_str.to_string()));
                }
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

        Ok(Args {
            paths,
            include_all,
            max_size,
            max_file_size,
            exclude_patterns,
            stdout,
        })
    }
}

/// Argument parsing errors
enum ArgsError {
    InvalidCount,
    HelpRequested,
    PathNotFound(PathBuf),
    InvalidSize(String),
    UnknownOption(String),
}

/// Print help message
fn print_help(program_name: &str) {
    eprintln!("{} v{}", AppInfo::NAME, AppInfo::VERSION);
    eprintln!("{}", AppInfo::DESCRIPTION);
    eprintln!();
    eprintln!("Usage: {} [OPTIONS] <path>...", program_name);
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --all, -a                   Include hidden directories and binary files");
    eprintln!("  --max-size, -m <size>       Set maximum output size (e.g., 10MB, 1GB, 500KB)");
    eprintln!("  --max-file-size, -f <size>  Skip files larger than this size (e.g., 500KB, 1MB)");
    eprintln!("  --exclude, -e <pattern>     Exclude files matching pattern (can be used multiple times)");
    eprintln!("  --stdout, -o                Output content to stdout instead of clipboard");
    eprintln!("  --help, -h                  Show this help message");
    eprintln!();
    eprintln!("Description:");
    eprintln!("  Recursively walks through directories, concatenates all file contents,");
    eprintln!("  and copies the result to the system clipboard (or outputs to stdout).");
    eprintln!();
    eprintln!("  You can specify multiple paths to process them all together.");
    eprintln!();
    eprintln!("  By default, hidden directories (starting with '.') and binary files");
    eprintln!("  are skipped. Use --all to include them.");
    eprintln!();
    eprintln!(
        "  The default size limit is {}. Use --max-size to change it.",
        ByteFormatter::format_as_unit(Config::DEFAULT_MAX_SIZE)
    );
    eprintln!(
        "  Files larger than {} are skipped by default. Use --max-file-size to change it.",
        ByteFormatter::format_as_unit(Config::DEFAULT_MAX_FILE_SIZE)
    );
    eprintln!();
    eprintln!("Examples:");
    eprintln!(
        "  {} src/                  # Process src directory",
        program_name
    );
    eprintln!(
        "  {} --all src/ tests/     # Include all files from both directories",
        program_name
    );
    eprintln!(
        "  {} --max-size 10MB src/  # Limit output to 10MB",
        program_name
    );
    eprintln!(
        "  {} --max-file-size 1MB src/  # Skip files larger than 1MB",
        program_name
    );
    eprintln!(
        "  {} -e '*.log' -e '*.tmp' src/  # Exclude log and tmp files",
        program_name
    );
    eprintln!(
        "  {} --exclude 'test_*' src/  # Exclude files starting with test_",
        program_name
    );
    eprintln!(
        "  {} --stdout src/ | less    # Output to stdout and pipe to less",
        program_name
    );
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
        ArgsError::InvalidSize(msg) => {
            eprintln!("Error: Invalid size - {}", msg);
        }
        ArgsError::UnknownOption(opt) => {
            eprintln!("Error: Unknown option '{}'", opt);
            eprintln!("Try '{} --help' for more information", program_name);
        }
        ArgsError::HelpRequested => {
            print_help(program_name);
        }
    }
}

fn main() {
    let program_name = env::args()
        .next()
        .unwrap_or_else(|| AppInfo::NAME.to_string());

    let args = match Args::parse() {
        Ok(args) => args,
        Err(error) => match error {
            ArgsError::HelpRequested => {
                print_help(&program_name);
                process::exit(0);
            }
            _ => {
                print_error(&program_name, error);
                process::exit(1);
            }
        },
    };

    // Validate clipboard utility is available before processing (unless using stdout)
    if !args.stdout {
        if let Err(error) = clipboard::validate_clipboard() {
            eprintln!("Error: {}", error);
            process::exit(1);
        }
    }

    run(args);
}

/// Run the application
fn run(args: Args) {
    let options = WalkOptions {
        include_all: args.include_all,
        max_size: args.max_size,
        max_file_size: args.max_file_size,
        exclude_patterns: args.exclude_patterns,
    };

    match walk_and_collect(&args.paths, options) {
        Ok(result) => {
            handle_result(result, args.max_size, args.stdout);
        }
        Err(error) => {
            eprintln!("Error: Failed to process directories - {}", error);
            process::exit(1);
        }
    }
}

/// Handle the collected result
fn handle_result(result: WalkResult, max_size: usize, stdout: bool) {
    let size = result.content.len();

    if size == 0 {
        if stdout {
            eprintln!("No files found to output");
        } else {
            eprintln!("No files found to copy");
        }
        return;
    }

    if stdout {
        // Output content to stdout
        print!("{}", result.content);
        
        // Status messages to stderr
        if result.truncated {
            eprintln!(
                "Content truncated at {} limit",
                ByteFormatter::format_as_unit(max_size)
            );
        }
        eprintln!(
            "Successfully output {} to stdout",
            ByteFormatter::format(size)
        );
        eprintln!("\n{}", result.stats.format_stats());
    } else {
        // Copy to clipboard (existing behavior)
        match clipboard::copy_to_clipboard(&result.content) {
            Ok(_) => {
                if result.truncated {
                    eprintln!(
                        "Content truncated at {} limit",
                        ByteFormatter::format_as_unit(max_size)
                    );
                    eprintln!(
                        "Successfully copied {} to clipboard",
                        ByteFormatter::format(size)
                    );
                } else {
                    eprintln!(
                        "Successfully copied {} to clipboard",
                        ByteFormatter::format(size)
                    );
                }
                eprintln!("\n{}", result.stats.format_stats());
            }
            Err(error) => {
                eprintln!("Error: Failed to copy to clipboard - {}", error);
                process::exit(1);
            }
        }
    }
}
