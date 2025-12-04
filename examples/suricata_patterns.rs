/// Example: Using regex_dfa with Suricata-style patterns
///
/// This demonstrates how to compile regex patterns into DFAs and use them
/// for pattern matching, similar to Suricata IDS rules.

use regex_dfa::Regex;

fn main() {
    println!("=== Regex DFA Pattern Matching Examples ===\n");

    // Example 1: Simple pattern matching
    example_basic_usage();

    // Example 2: Suricata CLSID patterns
    example_suricata_clsid_patterns();

    // Example 3: Handling complex patterns with memory limits
    example_bounded_compilation();

    // Example 4: Multiple pattern matching
    example_multiple_patterns();
}

/// Basic usage: compile and match
fn example_basic_usage() {
    println!("--- Example 1: Basic Usage ---");

    // Compile a simple pattern
    let pattern = r"\d{4}-\d{2}-\d{2}";
    match Regex::new(pattern) {
        Ok(re) => {
            let text = "Today's date is 2024-12-03";

            // Find the first match
            if let Some((start, end)) = re.find(text) {
                println!("Pattern: {}", pattern);
                println!("Text: {}", text);
                println!("Match found at position {}-{}: '{}'",
                         start, end, &text[start..end]);
            }

            // Check if text matches
            if re.is_match(text) {
                println!("✓ Pattern matches!\n");
            }
        }
        Err(e) => eprintln!("Failed to compile pattern: {}\n", e),
    }
}

/// Suricata CLSID patterns from your examples
fn example_suricata_clsid_patterns() {
    println!("--- Example 2: Suricata CLSID Patterns ---");

    // Pattern 1: Detect specific CLSID
    let pattern1 = r"<OBJECT\s+[^>]*classid\s*=\s*[\x22\x27]?\s*clsid\s*\x3a\s*\x7B?\s*66757BFC-DA0C-41E6-B3FE-B6D461223FF5";

    // Pattern 2: Simpler version for demonstration
    let pattern2 = r"<object\s+.*classid.*333C7BC4-460F-11D0-BC04-0080C7055A83";

    // Pattern 3: Very simple keyword match
    let pattern3 = r"(SnapshotPath|CompressedPath|PrintSnapshot)";

    // Test HTML snippets
    let html1 = r#"<OBJECT width="100" classid = "clsid:{66757BFC-DA0C-41E6-B3FE-B6D461223FF5}""#;
    let html2 = r#"<object classid="clsid:333C7BC4-460F-11D0-BC04-0080C7055A83">"#;
    let html3 = "Path: C:\\Snapshots\\SnapshotPath\\data.bin";

    // Try to compile and match pattern 1
    println!("\nPattern 1 (complex CLSID pattern):");
    match Regex::new(pattern1) {
        Ok(re) => {
            if let Some((start, end)) = re.find(html1) {
                println!("  ✓ Match found: '{}'", &html1[start..end]);
            } else {
                println!("  ✗ No match found");
            }
        }
        Err(e) => {
            println!("  ✗ Pattern too complex for DFA compilation: {}", e);
            println!("  (This is expected - character classes like [^>] can cause state explosion)");
        }
    }

    // Try pattern 2 (simplified)
    println!("\nPattern 2 (simplified CLSID pattern):");
    match Regex::new(pattern2) {
        Ok(re) => {
            if re.is_match(html2) {
                println!("  ✓ Pattern matches!");
                if let Some((start, end)) = re.find(html2) {
                    println!("  Match: '{}'", &html2[start..end]);
                }
            }
        }
        Err(e) => println!("  ✗ Failed: {}", e),
    }

    // Try pattern 3 (keywords)
    println!("\nPattern 3 (keyword alternation):");
    match Regex::new(pattern3) {
        Ok(re) => {
            if let Some((start, end)) = re.find(html3) {
                println!("  ✓ Match found: '{}'", &html3[start..end]);
            }
        }
        Err(e) => println!("  ✗ Failed: {}", e),
    }
    println!();
}

/// Using bounded compilation to limit memory usage
fn example_bounded_compilation() {
    println!("--- Example 3: Bounded Compilation ---");

    // This pattern might create too many DFA states
    let complex_pattern = r"<OBJECT\s+[^>]*classid";

    // Try with different state limits
    let limits = vec![100, 1000, 10000, 100000];

    for max_states in limits {
        match Regex::new_bounded(complex_pattern, max_states) {
            Ok(re) => {
                println!("  ✓ Compiled successfully with limit: {} states", max_states);

                let test = r#"<OBJECT width="50" classid="test""#;
                if re.is_match(test) {
                    println!("    Pattern matches test input");
                }
                break;
            }
            Err(e) => {
                println!("  ✗ Failed with limit {}: {}", max_states, e);
            }
        }
    }
    println!();
}

/// Matching multiple patterns
fn example_multiple_patterns() {
    println!("--- Example 4: Multiple Pattern Matching ---");

    // Compile multiple patterns
    let patterns = vec![
        ("CLSID-66757BFC", r"66757BFC-DA0C-41E6-B3FE-B6D461223FF5"),
        ("CLSID-E2883E8F", r"E2883E8F-472F-4fb0-9522-AC9BF37916A7"),
        ("CLSID-333C7BC4", r"333C7BC4-460F-11D0-BC04-0080C7055A83"),
        ("Path Keywords", r"(SnapshotPath|CompressedPath|PrintSnapshot)"),
    ];

    let mut compiled_patterns = Vec::new();

    for (name, pattern) in &patterns {
        match Regex::new(pattern) {
            Ok(re) => {
                compiled_patterns.push((*name, re));
                println!("  ✓ Compiled: {}", name);
            }
            Err(e) => {
                println!("  ✗ Failed to compile {}: {}", name, e);
            }
        }
    }

    // Test against multiple inputs
    let test_inputs = vec![
        r#"clsid:66757BFC-DA0C-41E6-B3FE-B6D461223FF5"#,
        r#"classid="clsid:{333C7BC4-460F-11D0-BC04-0080C7055A83}""#,
        r#"C:\Path\SnapshotPath\file.dat"#,
        r#"Normal text without matches"#,
    ];

    println!("\n  Testing inputs:");
    for (i, input) in test_inputs.iter().enumerate() {
        println!("\n  Input {}: '{}'", i + 1, input);

        let mut found_match = false;
        for (name, re) in &compiled_patterns {
            if let Some((start, end)) = re.find(input) {
                println!("    ✓ Matched by {}: '{}'", name, &input[start..end]);
                found_match = true;
            }
        }

        if !found_match {
            println!("    ✗ No matches");
        }
    }

    println!();
}
