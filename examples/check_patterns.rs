/// Check which patterns from a file are supported by regex-dfa
///
/// Usage:
/// ```bash
/// cargo run --example check_patterns -- /path/to/patterns.txt
/// cargo run --example check_patterns -- /path/to/patterns.txt --max-states 5000
/// ```

use regex_dfa::Regex;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cargo run --example check_patterns -- <patterns_file> [--max-states N]");
        std::process::exit(1);
    }

    let file_path = &args[1];

    // Parse optional max-states argument
    let max_states = if args.len() >= 4 && args[2] == "--max-states" {
        args[3].parse().unwrap_or(10000)
    } else {
        10000 // Default limit to avoid state explosion
    };

    let file = File::open(file_path).expect("Failed to open file");
    let reader = BufReader::new(file);

    let mut total = 0;
    let mut success = 0;
    let mut failed = 0;
    let mut skipped = 0;

    // Group errors by type
    let mut error_types: HashMap<String, Vec<(usize, String)>> = HashMap::new();

    let lines: Vec<_> = reader.lines().collect();
    let total_lines = lines.len();

    for (line_num, line) in lines.into_iter().enumerate() {
        let line = line.expect("Failed to read line");
        let line_num = line_num + 1; // 1-indexed

        // Skip empty lines and comments
        if line.trim().is_empty() || line.trim().starts_with('#') {
            skipped += 1;
            continue;
        }

        total += 1;

        // Print progress every 100 patterns
        if total % 100 == 0 {
            eprint!("\rProcessing: {}/{} patterns...", total, total_lines - skipped);
            std::io::stderr().flush().ok();
        }

        // Try to compile the pattern with bounded states
        match Regex::new_bounded(&line, max_states) {
            Ok(_) => {
                success += 1;
            }
            Err(e) => {
                failed += 1;
                let error_str = format!("{}", e);
                // Extract error type (first line or first part)
                let error_type = error_str.lines().next().unwrap_or(&error_str).to_string();

                error_types
                    .entry(error_type)
                    .or_insert_with(Vec::new)
                    .push((line_num, line.clone()));
            }
        }
    }

    eprintln!("\rProcessing complete.                    ");

    println!("=== Pattern Compatibility Report ===\n");
    println!("File: {}", file_path);
    println!("Max states: {}", max_states);
    println!("Total patterns: {}", total);
    println!("Successful: {} ({:.1}%)", success, 100.0 * success as f64 / total as f64);
    println!("Failed: {} ({:.1}%)", failed, 100.0 * failed as f64 / total as f64);
    println!("Skipped (comments/empty): {}", skipped);
    println!();

    if !error_types.is_empty() {
        println!("=== Errors by Type ===\n");

        // Sort by frequency
        let mut sorted_errors: Vec<_> = error_types.iter().collect();
        sorted_errors.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        for (error_type, patterns) in sorted_errors {
            println!("Error: {} ({} occurrences)", error_type, patterns.len());
            // Show first 3 examples
            for (line_num, pattern) in patterns.iter().take(3) {
                let truncated = if pattern.len() > 60 {
                    format!("{}...", &pattern[..60])
                } else {
                    pattern.clone()
                };
                println!("  Line {}: {}", line_num, truncated);
            }
            if patterns.len() > 3 {
                println!("  ... and {} more", patterns.len() - 3);
            }
            println!();
        }
    }
}
