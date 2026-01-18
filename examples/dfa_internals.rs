/// Example: Exploring DFA compilation internals
///
/// This demonstrates the step-by-step compilation pipeline from regex to DFA,
/// showing the internal structure at each stage.
///
/// # Usage
///
/// Inspect a specific pattern:
/// ```bash
/// cargo run --example dfa_internals -- "hello"
/// cargo run --example dfa_internals -- "\d{4}-\d{2}-\d{2}"
/// cargo run --example dfa_internals -- "\bword\b"
/// ```
///
/// Run with built-in examples:
/// ```bash
/// cargo run --example dfa_internals
/// ```
///
/// Set maximum states limit (default: 10000):
/// ```bash
/// cargo run --example dfa_internals -- "pattern" --max-states 5000
/// ```
///
/// Show only NFA stages (steps 1 and 2):
/// ```bash
/// cargo run --example dfa_internals -- "a?b" --nfa
/// ```
///
/// # What This Shows
///
/// For each pattern, this example displays:
/// 1. **NFA with looks** - Initial NFA with lookaround assertions (word boundaries, anchors)
/// 2. **NFA without looks** - After removing lookarounds via the `remove_looks()` algorithm
/// 3. **Byte NFA** - After converting from Unicode codepoints to UTF-8 bytes
/// 4. **Raw DFA** - After determinization via subset construction
/// 5. **Optimized DFA** - After minimization using Hopcroft's algorithm
/// 6. **Prefix extraction** - Common prefix strings for optimization
///
/// # Understanding the Output
///
/// - **State numbers**: 0-indexed states in the automaton
/// - **Transitions**: Byte ranges that trigger state transitions (e.g., "104 -- 104" is 'h')
/// - **Accept states**: States where a match is found
/// - **Init states**: Starting states for different lookaround contexts
/// - **Look types**: Full, WordChar, NotWordChar, NewLine, Boundary

use regex_dfa::nfa::Nfa;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse command line arguments
    let mut pattern_arg: Option<String> = None;
    let mut max_states = 10000;
    let mut nfa_only = false;
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--max-states" => {
                if i + 1 < args.len() {
                    max_states = args[i + 1].parse().unwrap_or_else(|_| {
                        eprintln!("Invalid max-states value: {}", args[i + 1]);
                        std::process::exit(1);
                    });
                    i += 2;
                } else {
                    eprintln!("--max-states requires a value");
                    std::process::exit(1);
                }
            }
            "--nfa" => {
                nfa_only = true;
                i += 1;
            }
            "--help" | "-h" => {
                print_usage();
                return;
            }
            arg => {
                if pattern_arg.is_none() {
                    pattern_arg = Some(arg.to_string());
                } else {
                    eprintln!("Multiple patterns provided. Please provide only one pattern.");
                    std::process::exit(1);
                }
                i += 1;
            }
        }
    }

    println!("=== DFA Compilation Pipeline ===\n");

    // If a pattern was provided, analyze it
    if let Some(pattern) = pattern_arg {
        compile_and_show_dfa(&pattern, "User-provided pattern", max_states, nfa_only);
    } else {
        // Run built-in examples
        println!("Running built-in examples. Use --help for usage information.\n");

        // Example 1: Simple pattern
        example_simple_pattern();

        // Example 2: Word boundary pattern
        example_word_boundary();

        // Example 3: Date pattern
        example_date_pattern();

        // Example 4: Alternation pattern
        example_alternation();
    }
}

fn print_usage() {
    println!("DFA Internals Explorer");
    println!();
    println!("Usage:");
    println!("  cargo run --example dfa_internals                  Run built-in examples");
    println!("  cargo run --example dfa_internals -- <pattern>     Analyze a specific pattern");
    println!();
    println!("Options:");
    println!("  --nfa               Show only NFA stages (steps 1 and 2)");
    println!("  --max-states <n>    Set maximum DFA states (default: 10000)");
    println!("  --help, -h          Show this help message");
    println!();
    println!("Examples:");
    println!("  cargo run --example dfa_internals -- \"hello\"");
    println!("  cargo run --example dfa_internals -- \"\\d{{4}}-\\d{{2}}-\\d{{2}}\"");
    println!("  cargo run --example dfa_internals -- \"\\bword\\b\"");
    println!("  cargo run --example dfa_internals -- \"(cat|dog|bird)\" --max-states 5000");
    println!("  cargo run --example dfa_internals -- \"a?b\" --nfa");
}

/// Simple pattern: literal string
fn example_simple_pattern() {
    println!("--- Example 1: Simple Literal Pattern ---");
    let pattern = r"hello";

    compile_and_show_dfa(pattern, "Simple literal string", 10000, false);
}

/// Pattern with word boundaries
fn example_word_boundary() {
    println!("\n--- Example 2: Word Boundary Pattern ---");
    let pattern = r"\bword\b";

    compile_and_show_dfa(pattern, "Word boundary pattern", 10000, false);
}

/// Date pattern with digits
fn example_date_pattern() {
    println!("\n--- Example 3: Date Pattern ---");
    let pattern = r"\d{4}-\d{2}-\d{2}";

    compile_and_show_dfa(pattern, "Date pattern (YYYY-MM-DD)", 10000, false);
}

/// Alternation pattern
fn example_alternation() {
    println!("\n--- Example 4: Alternation Pattern ---");
    let pattern = r"(cat|dog|bird)";

    compile_and_show_dfa(pattern, "Alternation of keywords", 10000, false);
}

/// Compile a pattern and show the DFA structure at each stage
fn compile_and_show_dfa(pattern: &str, description: &str, max_states: usize, nfa_only: bool) {
    println!("Pattern: {}", pattern);
    println!("Description: {}", description);
    if !nfa_only {
        println!("Max states: {}", max_states);
    }
    println!();

    // Step 1: Parse regex into NFA with looks
    println!("Step 1: Parse regex into NFA (with lookarounds)");
    let nfa_with_looks = match Nfa::from_regex(pattern) {
        Ok(nfa) => nfa,
        Err(e) => {
            println!("  ✗ Failed to parse regex: {}\n", e);
            return;
        }
    };
    println!("  ✓ Created NFA with {} states", nfa_with_looks.num_states());
    println!("{:#?}", nfa_with_looks);
    println!();

    // Step 2: Remove lookarounds
    println!("Step 2: Remove lookaround assertions");
    let nfa_no_looks = nfa_with_looks.remove_looks();
    println!("  ✓ NFA after removing looks: {} states", nfa_no_looks.num_states());
    println!("{:#?}", nfa_no_looks);
    println!();

    // If --nfa flag is set, stop here
    if nfa_only {
        println!("(Stopped after NFA stages due to --nfa flag)");
        return;
    }

    // Step 3: Convert to byte-based NFA
    println!("Step 3: Convert to byte-based NFA (UTF-8)");
    let byte_nfa = match nfa_no_looks.byte_me(max_states) {
        Ok(nfa) => nfa,
        Err(e) => {
            println!("  ✗ Failed to convert to byte NFA: {}\n", e);
            return;
        }
    };
    println!("  ✓ Byte NFA: {} states", byte_nfa.num_states());
    println!("{:#?}", byte_nfa);
    println!();

    // Step 4: Determinize (NFA -> DFA)
    println!("Step 4: Determinize (NFA → DFA via subset construction)");
    let dfa = match byte_nfa.determinize(max_states) {
        Ok(dfa) => dfa,
        Err(e) => {
            println!("  ✗ Failed to determinize: {}\n", e);
            return;
        }
    };
    println!("  ✓ Raw DFA: {} states", dfa.num_states());
    println!("{:#?}", dfa);
    println!();

    // Step 5: Optimize (minimize states)
    println!("Step 5: Optimize (minimize DFA states)");
    let optimized_dfa = dfa.optimize();
    println!("  ✓ Optimized DFA: {} states", optimized_dfa.num_states());
    println!("{:#?}", optimized_dfa);
    println!();

    // Step 6: Show prefix extraction
    println!("Step 6: Extract common prefixes");
    let prefix_strings = optimized_dfa.prefix_strings();
    if prefix_strings.is_empty() {
        println!("  No common prefixes found");
    } else {
        println!("  Common prefix parts:");
        for part in prefix_strings {
            print!("    \"");
            for byte in &part.0 {
                if *byte >= 32 && *byte < 127 {
                    print!("{}", *byte as char);
                } else {
                    print!("\\x{:02x}", byte);
                }
            }
            println!("\" (leads to state {})", part.1);
        }
    }
    println!();

    // Summary
    println!("Summary:");
    println!("  Pattern: {}", pattern);
    println!("  Final DFA states: {}", optimized_dfa.num_states());
    println!("  Memory efficient: {}",
             if optimized_dfa.num_states() < 50 { "Yes" } else { "Consider limiting" });
    println!("{}", "=".repeat(70));
}
