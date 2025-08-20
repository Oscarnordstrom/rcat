# rcat - Recursive Cat

A fast, multi-threaded command-line utility that recursively concatenates files and copies them to your clipboard.

## What is rcat?

`rcat` (Recursive Cat) walks through directory structures, reads all files, concatenates their contents with clear file markers, and automatically copies the result to your system clipboard. It's designed for quickly gathering code, logs, or text files for analysis, review, or sharing.

### Key Features

- **Recursive directory traversal** with multi-threaded processing
- **Automatic clipboard integration** (cross-platform support)
- **Binary file detection** - marks binary files without attempting to read them
- **Size limiting** - enforces 1GB limit to prevent memory issues
- **Performance statistics** - displays processing speed and file counts
- **Smart formatting** - adds clear file path headers for easy navigation

## Installation

### Prerequisites

- Rust toolchain (install from [rustup.rs](https://rustup.rs/))
- Clipboard utility:
  - macOS: `pbcopy` (included by default)
  - Linux: `xclip` (install with `sudo apt-get install xclip` or equivalent)
  - Windows: `clip` (included by default)

### Quick Install (For unix like systems)

```bash
# Clone the repository
git clone git@github.com:Oscarnordstrom/rcat.git
cd rcat

# Use the installation script
./install-local.sh
```

This will:
1. Build the project in release mode
2. Install the binary to `~/.local/bin/rcat`
3. Provide instructions to add `~/.local/bin` to your PATH if needed

## Usage

```bash
# Process a single file
rcat file.txt

# Process an entire directory
rcat /path/to/project

# Process current directory
rcat .
```

The output is automatically copied to your clipboard and statistics are displayed:

```
Processing complete! Output copied to clipboard.

Statistics:
  Files processed: 42
  Directories: 5
  Binary files: 3
  Total size: 256.3 KB
  Time: 0.15s (280 files/sec, 1.7 MB/sec)
```

## Output Format

Files are concatenated with clear headers:

```
--- /path/to/file.txt ---
[file content here]

--- /path/to/binary.exe ---
<BINARY_FILE>
```
