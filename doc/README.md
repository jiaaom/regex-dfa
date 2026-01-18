# regex-dfa Documentation

Welcome to the comprehensive documentation for `regex-dfa`, a Rust crate that compiles regular expressions into deterministic finite automata (DFAs).

## Overview

Unlike lazy DFA compilation (as used in Rust's standard `regex` crate), `regex-dfa` performs **eager compilation**. This means:

- **Trade-off**: Increased memory usage and compilation time
- **Benefit**: Faster and more consistent matching performance
- **Use case**: When regex matching is performance-critical and patterns are known ahead of time

## Quick Start

### Basic Usage

```rust
use regex_dfa::Regex;

fn main() {
    // Create a regex
    let re = Regex::new(r"\d+").unwrap();

    // Check if a string matches
    if re.is_match("hello 123 world") {
        println!("Found a match!");
    }

    // Find the first match (returns byte indices)
    if let Some((start, end)) = re.find("hello 123 world") {
        println!("Match found at bytes {}..{}", start, end);
        // Output: Match found at bytes 6..9
    }
}
```

### Bounded Compilation

For untrusted patterns or resource-constrained environments, use bounded compilation to limit memory usage:

```rust
use regex_dfa::Regex;

fn main() {
    // Limit DFA to 10,000 states maximum
    match Regex::new_bounded(r"complex|pattern", 10_000) {
        Ok(re) => {
            // Use the regex
        }
        Err(e) => {
            // Handle error (may be TooManyStates)
            eprintln!("Failed to compile regex: {}", e);
        }
    }
}
```

## Documentation Index

| Document | Description |
|----------|-------------|
| [Architecture](./architecture.md) | High-level architecture and compilation pipeline |
| [API Reference](./api-reference.md) | Public types and methods |
| [Advanced Usage](./advanced-usage.md) | Direct NFA/DFA manipulation and custom automata |
| [Algorithms](./algorithms.md) | Internal algorithms (lookaround removal, determinization, minimization) |
| [Memory & Performance](./memory-and-performance.md) | Memory management and performance considerations |

## Key Features

### Eager DFA Compilation

All possible DFA states are computed at compile time, resulting in:
- O(n) matching time where n is input length
- No backtracking during matching
- Consistent performance regardless of input content

### Lookaround Support

The library supports:
- Word boundaries (`\b`, `\B`)
- Line anchors (`^`, `$`)
- Start/end anchors

### Unicode Support

Full Unicode support including:
- Unicode character classes (`\w`, `\d`, `\s` and their negations)
- UTF-8 encoding handled transparently
- Multi-byte character sequences

### Memory Control

Protect against pathological regexes with `new_bounded()`:
- Set maximum state limits
- Early failure for patterns that would exceed limits
- Critical for accepting user-provided patterns

## Comparison with `regex` Crate

| Aspect | `regex` | `regex-dfa` |
|--------|---------|-------------|
| Compilation | Lazy (on-demand) | Eager (upfront) |
| Memory | Lower baseline, grows with input | Higher baseline, constant |
| Match time | Generally fast, occasional slowdowns | Consistent O(n) |
| Backtracking | Never (but may compute states lazily) | Never |
| Best for | General use | Performance-critical matching |

## Project Status

This codebase has been successfully ported from Rust 2015 (circa 2016) to Rust 2021 and modern stable Rust. Key changes in the port:
- Updated to Rust 2021 edition
- Replaced `try!` macros with `?` operator
- Added `dyn` keyword to trait objects
- Updated module paths to use `crate::` prefix
- All 45 tests passing

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT License

at your option.
