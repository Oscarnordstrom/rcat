# rcat

Recursively concatenates files from directories and copies to clipboard or outputs to stdout.

## Motivation

When working with multiple files across different directories, I found myself needing to quickly copy their contents together for analysis, sharing, or processing. Simple bash scripts kept including files I didn't want (logs, build artifacts, etc.) and were unreliable. I built this tool to provide a simple interface to quickly copy project content to my clipboard.

## What it does

Walks directory trees, reads all text files, concatenates them with file headers, and copies the result to your clipboard (or outputs to stdout). Binary files are detected and marked. Hidden files/directories and gitignored paths are skipped by default.

## Features

### **Recursive Directory Traversal**
Walks through directories in breadth-first order, processing all files systematically.

### **Flexible Output**
Results go to your clipboard by default, or output to stdout for piping and redirection.

### **Smart File Filtering**
- **Gitignore Support**: Respects .gitignore files hierarchically
- **Hidden File Filtering**: Skips hidden files/directories by default
- **Binary Detection**: Identifies and marks binary files
- **Size Limits**: Skip files over a certain size (500KB default)
- **Custom Exclusions**: Use patterns to exclude specific files

### **Flexible Input**
Process single directories, multiple paths, or current directory.

### **Progress Statistics**
Shows what was processed, skipped, and why.

## Usage

```bash
# Single directory
rcat src/

# Multiple paths  
rcat src/ tests/ docs/

# Current directory
rcat .

# Include hidden files and binary content
rcat --all src/

# Set custom size limit
rcat --max-size 10MB src/

# Skip files larger than 1MB
rcat --max-file-size 1MB src/

# Exclude specific patterns
rcat -e '*.log' -e '*.tmp' src/

# Multiple exclusions
rcat --exclude '*.rs' --exclude 'test_*' --exclude '*.json' src/

# Output to stdout instead of clipboard
rcat --stdout src/
rcat -o src/

# Pipe to other commands
rcat -o src/ | less
rcat -o src/ | grep "TODO"
rcat -o src/ | wc -l

# Redirect to file
rcat -o src/ > combined.txt
```

## Options

- `--all, -a` - Include hidden directories and binary files
- `--max-size, -m <size>` - Set maximum output size (e.g., 10MB, 1GB, 500KB)
- `--max-file-size, -f <size>` - Skip files larger than this size (e.g., 500KB, 1MB)
- `--exclude, -e <pattern>` - Exclude files matching pattern (can be used multiple times)
- `--stdout, -o` - Output content to stdout instead of clipboard
- `--help, -h` - Show help message

**Size formats**: Use human-readable sizes like `500KB`, `10MB`, `1GB`

**Exclude patterns**: Use glob patterns like `*.log`, `test_*`, `config.yaml`

## Installation

### macOS/Linux

```bash
git clone https://github.com/oscarnordstrom/rcat.git
cd rcat
./install.sh
```

### Windows

TODO: Windows installation instructions

### Manual Installation

```bash
cargo build --release
cp target/release/rcat ~/.local/bin/
# Add ~/.local/bin to your PATH
```

## Requirements

- Rust toolchain
- Clipboard utility:
  - macOS: `pbcopy` (built-in)
  - Linux: `xclip` (`sudo apt install xclip` or `sudo pacman -S xclip`)
  - Windows: `clip` (built-in)

## Default Behavior

- **Size limits**: 5MB total output, 500KB per file
- **Skips**: Hidden files, binary files, gitignored paths
- **Includes**: Text files in current directory and subdirectories
- **Order**: Breadth-first traversal (files at same level before going deeper)