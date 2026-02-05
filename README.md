# dedup

A fast and simple file deduplicator written in Rust. Find and remove duplicate files based on SHA-256 hash comparison.

## Features

- Fast parallel file hashing
- Beautiful colored output with progress bars
- Recursive directory scanning
- Shows wasted space by duplicates
- Safe deletion with interactive prompts
- Dry-run mode to preview deletions
- Configurable minimum file size filter
- Option to skip hidden files

## Installation

### From source

```bash
git clone https://github.com/heyrtl/dedup.git
cd dedup
cargo build --release
```

The binary will be in `target/release/dedup`

### Using cargo

```bash
cargo install --path .
```

## Usage

### Scan a directory for duplicates

```bash
dedup /path/to/directory
```

### Preview what would be deleted (dry-run)

```bash
dedup /path/to/directory --dry-run
```

### Interactively delete duplicates

```bash
dedup /path/to/directory --delete
```

### Advanced options

```bash
# Only consider files larger than 1MB
dedup /path/to/directory --min-size 1048576

# Include hidden files
dedup /path/to/directory --skip-hidden=false

# Combine options
dedup ~/Downloads --delete --min-size 10000
```

## Examples

**Finding duplicates:**

```bash
$ dedup ~/Downloads

Scanning directory: /home/user/Downloads

Found: 1,247 files

Duplicate Groups Found:

Group 1: 450 MB (3 files, 150 MB each, 300 MB wasted)
  1. /home/user/Downloads/movie.mp4
  2. /home/user/Downloads/movie-copy.mp4
  3. /home/user/Downloads/movie-2.mp4

Group 2: 24 MB (2 files, 12 MB each, 12 MB wasted)
  1. /home/user/Downloads/photo.jpg
  2. /home/user/Downloads/photo-backup.jpg

Summary:
  4 duplicate files in 2 groups
  312 MB wasted space

Run with --delete to interactively remove duplicates
Run with --dry-run to see what would be deleted
```

**Interactive deletion:**

```bash
$ dedup ~/Downloads --delete

Group 1 (150 MB)
  1. /home/user/Downloads/movie.mp4
  2. /home/user/Downloads/movie-copy.mp4
  3. /home/user/Downloads/movie-2.mp4

Enter numbers to delete (space-separated, or 'a' for all except first, 's' to skip): 2 3
  ✓ Deleted: /home/user/Downloads/movie-copy.mp4
  ✓ Deleted: /home/user/Downloads/movie-2.mp4

Complete: 2 files deleted, 300 MB freed
```

## How it works

1. Recursively scans the specified directory
2. Computes SHA-256 hash for each file
3. Groups files with identical hashes
4. Displays duplicate groups sorted by wasted space
5. Optionally deletes selected duplicates interactively

## Options

```
Usage: dedup [OPTIONS] <PATH>

Arguments:
  <PATH>  Directory to scan for duplicates

Options:
  -d, --delete              Actually delete files (interactive mode)
  -n, --dry-run             Show what would be deleted without deleting
  -m, --min-size <MIN_SIZE> Minimum file size to consider (in bytes) [default: 1]
      --skip-hidden         Skip hidden files and directories [default: true]
  -h, --help                Print help
```

## Safety

- Uses interactive prompts before any deletion
- Dry-run mode available for safe previewing
- Only deletes files you explicitly select
- Preserves at least one copy of each duplicate group

## License

MIT

## Contributing

Pull requests are welcome! For major changes, please open an issue first to discuss what you would like to change.

## Author

Built with Rust 🦀# dedup
