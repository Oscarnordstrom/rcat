# rcat

Recursively concatenates files from directories and copies to clipboard.

## What it does

Walks directory trees, reads all text files, concatenates them with file headers, and copies the result to your clipboard. Binary files are detected and marked. Hidden files/directories and gitignored paths are skipped by default.

## Features

- Recursive directory traversal (breadth-first)
- Automatic clipboard copy
- Binary file detection
- Gitignore support (hierarchical)
- Hidden file/directory filtering
- Size limiting with configurable max
- Multiple path support
- Progress statistics

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
```

## Options

- `--all, -a` - Include hidden directories and binary files
- `--max-size, -m <size>` - Set maximum output size (e.g., 10MB, 1GB, 500KB)
- `--help, -h` - Show help message

## Installation

### macOS/Linux

```bash
git clone https://github.com/username/rcat.git
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
  - Linux: `xclip` (`sudo apt install xclip`)
  - Windows: `clip` (built-in)