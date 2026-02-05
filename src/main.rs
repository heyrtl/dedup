use clap::Parser;
use colored::Colorize;
use humansize::{format_size, DECIMAL};
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "dedup")]
#[command(about = "Find and remove duplicate files", long_about = None)]
struct Cli {
    /// Directory to scan for duplicates
    path: PathBuf,

    /// Actually delete files (interactive mode)
    #[arg(short, long)]
    delete: bool,

    /// Show what would be deleted without deleting
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Minimum file size to consider (in bytes)
    #[arg(short, long, default_value = "1")]
    min_size: u64,

    /// Skip hidden files and directories
    #[arg(long, default_value = "true")]
    skip_hidden: bool,
}

#[derive(Debug)]
struct FileInfo {
    path: PathBuf,
    size: u64,
}

fn main() {
    let cli = Cli::parse();

    if !cli.path.exists() {
        eprintln!("{} Path does not exist: {}", "Error:".red().bold(), cli.path.display());
        std::process::exit(1);
    }

    if !cli.path.is_dir() {
        eprintln!("{} Path is not a directory: {}", "Error:".red().bold(), cli.path.display());
        std::process::exit(1);
    }

    println!("{} {}", "Scanning directory:".cyan().bold(), cli.path.display());
    println!();

    // Collect all files
    let files = collect_files(&cli.path, cli.min_size, cli.skip_hidden);
    
    if files.is_empty() {
        println!("{}", "No files found.".yellow());
        return;
    }

    println!("{} {} files", "Found:".green().bold(), files.len());
    println!();

    // Hash files and find duplicates
    let duplicates = find_duplicates(files);

    if duplicates.is_empty() {
        println!("{}", "No duplicates found!".green().bold());
        return;
    }

    // Calculate statistics
    let (duplicate_count, wasted_space) = calculate_stats(&duplicates);

    println!("{}", "Duplicate Groups Found:".red().bold());
    println!();

    // Display duplicates
    display_duplicates(&duplicates);

    println!();
    println!("{}", "Summary:".cyan().bold());
    println!("  {} duplicate files in {} groups", duplicate_count, duplicates.len());
    println!("  {} wasted space", format_size(wasted_space, DECIMAL).yellow().bold());
    println!();

    // Handle deletion
    if cli.delete || cli.dry_run {
        handle_deletion(&duplicates, cli.dry_run);
    } else {
        println!("{}", "Run with --delete to interactively remove duplicates".dimmed());
        println!("{}", "Run with --dry-run to see what would be deleted".dimmed());
    }
}

fn collect_files(path: &Path, min_size: u64, skip_hidden: bool) -> Vec<FileInfo> {
    let mut files = Vec::new();

    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip hidden files if requested
        if skip_hidden {
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().starts_with('.') {
                    continue;
                }
            }
        }

        // Only process files
        if path.is_file() {
            if let Ok(metadata) = fs::metadata(path) {
                let size = metadata.len();
                if size >= min_size {
                    files.push(FileInfo {
                        path: path.to_path_buf(),
                        size,
                    });
                }
            }
        }
    }

    files
}

fn find_duplicates(files: Vec<FileInfo>) -> HashMap<String, Vec<FileInfo>> {
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut hash_map: HashMap<String, Vec<FileInfo>> = HashMap::new();

    for file in files {
        pb.set_message(format!("Hashing: {}", file.path.display()));
        
        match hash_file(&file.path) {
            Ok(hash) => {
                hash_map.entry(hash).or_insert_with(Vec::new).push(file);
            }
            Err(e) => {
                eprintln!("Error hashing {}: {}", file.path.display(), e);
            }
        }
        
        pb.inc(1);
    }

    pb.finish_with_message("Hashing complete");

    // Filter out non-duplicates
    hash_map.retain(|_, files| files.len() > 1);
    hash_map
}

fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn calculate_stats(duplicates: &HashMap<String, Vec<FileInfo>>) -> (usize, u64) {
    let mut duplicate_count = 0;
    let mut wasted_space = 0u64;

    for files in duplicates.values() {
        if let Some(first) = files.first() {
            let size = first.size;
            let extra_copies = files.len() - 1;
            duplicate_count += extra_copies;
            wasted_space += size * extra_copies as u64;
        }
    }

    (duplicate_count, wasted_space)
}

fn display_duplicates(duplicates: &HashMap<String, Vec<FileInfo>>) {
    let mut groups: Vec<_> = duplicates.iter().collect();
    groups.sort_by(|a, b| b.1[0].size.cmp(&a.1[0].size));

    for (i, (_hash, files)) in groups.iter().enumerate() {
        let size = files[0].size;
        let wasted = size * (files.len() - 1) as u64;

        println!(
            "{} {} ({} files, {} each, {} wasted)",
            format!("Group {}:", i + 1).yellow().bold(),
            format_size(wasted, DECIMAL).red(),
            files.len(),
            format_size(size, DECIMAL),
            format_size(wasted, DECIMAL)
        );

        for (j, file) in files.iter().enumerate() {
            println!("  {}. {}", j + 1, file.path.display().to_string().dimmed());
        }
        println!();
    }
}

fn handle_deletion(duplicates: &HashMap<String, Vec<FileInfo>>, dry_run: bool) {
    let mut groups: Vec<_> = duplicates.iter().collect();
    groups.sort_by(|a, b| b.1[0].size.cmp(&a.1[0].size));

    let mut total_deleted = 0;
    let mut total_freed = 0u64;

    for (i, (_hash, files)) in groups.iter().enumerate() {
        println!(
            "\n{} ({})",
            format!("Group {}", i + 1).yellow().bold(),
            format_size(files[0].size, DECIMAL)
        );

        for (j, file) in files.iter().enumerate() {
            println!("  {}. {}", j + 1, file.path.display());
        }

        println!();
        print!("Enter numbers to {} (space-separated, or 'a' for all except first, 's' to skip): ", 
               if dry_run { "mark for deletion" } else { "delete" });
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input == "s" || input.is_empty() {
            continue;
        }

        let indices_to_delete: Vec<usize> = if input == "a" {
            (1..files.len()).collect()
        } else {
            input
                .split_whitespace()
                .filter_map(|s| s.parse::<usize>().ok())
                .filter(|&i| i > 0 && i <= files.len())
                .map(|i| i - 1)
                .collect()
        };

        for &idx in &indices_to_delete {
            let file = &files[idx];
            
            if dry_run {
                println!("  {} Would delete: {}", "✓".green(), file.path.display());
                total_deleted += 1;
                total_freed += file.size;
            } else {
                match fs::remove_file(&file.path) {
                    Ok(_) => {
                        println!("  {} Deleted: {}", "✓".green(), file.path.display());
                        total_deleted += 1;
                        total_freed += file.size;
                    }
                    Err(e) => {
                        eprintln!("  {} Failed to delete {}: {}", "✗".red(), file.path.display(), e);
                    }
                }
            }
        }
    }

    println!();
    if dry_run {
        println!(
            "{} {} files would be deleted, {} would be freed",
            "Dry run complete:".cyan().bold(),
            total_deleted,
            format_size(total_freed, DECIMAL).yellow().bold()
        );
    } else {
        println!(
            "{} {} files deleted, {} freed",
            "Complete:".green().bold(),
            total_deleted,
            format_size(total_freed, DECIMAL).yellow().bold()
        );
    }
}