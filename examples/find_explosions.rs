/// Find patterns that cause state explosion and show details
///
/// Usage:
/// ```bash
/// cargo run --example find_explosions -- /path/to/patterns.txt
/// ```

use regex_dfa::nfa::Nfa;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cargo run --example find_explosions -- <patterns_file>");
        std::process::exit(1);
    }

    let file_path = &args[1];
    let file = File::open(file_path).expect("Failed to open file");
    let reader = BufReader::new(file);

    let max_states = 10000;
    let mut count = 0;

    println!("=== Patterns Causing State Explosion ===\n");

    for (line_num, line) in reader.lines().enumerate() {
        let line = line.expect("Failed to read line");
        let line_num = line_num + 1;

        if line.trim().is_empty() || line.trim().starts_with('#') {
            continue;
        }

        // Try to parse and trace where explosion happens
        let nfa_with_looks = match Nfa::from_regex(&line) {
            Ok(nfa) => nfa,
            Err(_) => continue, // Skip syntax errors
        };

        let nfa_no_looks = nfa_with_looks.remove_looks();
        let nfa_states_before_bytes = nfa_no_looks.num_states();

        let byte_nfa = match nfa_no_looks.byte_me(max_states) {
            Ok(nfa) => nfa,
            Err(_) => {
                count += 1;
                println!("Pattern #{} (line {}): EXPLODED at byte_me()", count, line_num);
                println!("  Pattern: {}", if line.len() > 80 { format!("{}...", &line[..80]) } else { line.clone() });
                println!("  NFA states (before byte conversion): {}", nfa_states_before_bytes);
                println!();
                if count >= 10 { break; }
                continue;
            }
        };

        let byte_nfa_states = byte_nfa.num_states();

        match byte_nfa.determinize(max_states) {
            Ok(_) => {} // Success, not interesting
            Err(_) => {
                count += 1;
                println!("Pattern #{} (line {}): EXPLODED at determinize()", count, line_num);
                println!("  Pattern: {}", if line.len() > 80 { format!("{}...", &line[..80]) } else { line.clone() });
                println!("  NFA states (codepoints): {}", nfa_states_before_bytes);
                println!("  NFA states (bytes): {}", byte_nfa_states);
                println!();
                if count >= 10 { break; }
            }
        }
    }

    println!("Found {} patterns with state explosion (showing first 10)", count);
}
