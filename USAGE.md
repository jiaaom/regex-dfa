# Usage Guide for regex_dfa

## Quick Start

```rust
use regex_dfa::Regex;

fn main() {
    // Compile a regex pattern into a DFA
    let re = Regex::new(r"\d{4}-\d{2}-\d{2}").unwrap();

    // Find the first match
    if let Some((start, end)) = re.find("Date: 2024-12-03") {
        println!("Found match at position {}-{}", start, end);
    }

    // Check if pattern matches
    if re.is_match("2024-12-03") {
        println!("Pattern matches!");
    }
}
```

## API Overview

### Creating a Regex

```rust
// Compile with unlimited states (may fail with TooManyStates error)
let re = Regex::new(r"pattern")?;

// Compile with a state limit (recommended for complex patterns)
let re = Regex::new_bounded(r"pattern", 10000)?;
```

### Matching

```rust
// Find first match - returns (start_byte_index, end_byte_index)
let result: Option<(usize, usize)> = re.find(text);

// Check if text matches (faster, doesn't return position)
let matches: bool = re.is_match(text);
```

## Using with Suricata Patterns

Your Suricata patterns can work with this crate, but there are important considerations:

### ✅ What Works Well

**Simple CLSID patterns:**
```rust
let pattern = r"66757BFC-DA0C-41E6-B3FE-B6D461223FF5";
let re = Regex::new(pattern)?;
```

**Keyword alternation:**
```rust
let pattern = r"(SnapshotPath|CompressedPath|PrintSnapshot)";
let re = Regex::new(pattern)?;
```

**Simple bounded patterns:**
```rust
let pattern = r"<object.*classid.*333C7BC4";
let re = Regex::new(pattern)?;
```

### ⚠️ Potential Issues

**1. Character classes like `[^>]` can cause state explosion:**
```rust
// This might fail with "State overflow" error
let pattern = r"<OBJECT\s+[^>]*classid";
let re = Regex::new(pattern); // May fail!

// Solution: Use bounded compilation
let re = Regex::new_bounded(pattern, 100000)?;
```

**2. Hex escapes work, but use bounded compilation:**
```rust
// Your pattern with \x22 (hex for quote) works fine
let pattern = r"[\x22\x27]"; // Matches " or '
let re = Regex::new(pattern)?; // Works!
```

**3. Complex patterns with many alternations:**
```rust
// This creates many DFA states
let pattern = r"(word1|word2|word3|...|word100)";

// Use bounded compilation
let re = Regex::new_bounded(pattern, 50000)?;
```

## Practical Example: Suricata CLSID Detection

```rust
use regex_dfa::Regex;

fn detect_malicious_clsid(html: &str) -> Vec<&str> {
    let dangerous_clsids = vec![
        ("IE ActiveX", r"66757BFC-DA0C-41E6-B3FE-B6D461223FF5"),
        ("Windows Update", r"E2883E8F-472F-4fb0-9522-AC9BF37916A7"),
        ("DirectAnimation", r"333C7BC4-460F-11D0-BC04-0080C7055A83"),
    ];

    let mut matches = Vec::new();

    for (name, clsid) in dangerous_clsids {
        // Compile the pattern
        if let Ok(re) = Regex::new(clsid) {
            // Check if HTML contains this CLSID
            if re.is_match(html) {
                matches.push(name);
            }
        }
    }

    matches
}

fn main() {
    let html = r#"
        <OBJECT classid="clsid:{66757BFC-DA0C-41E6-B3FE-B6D461223FF5}">
        </OBJECT>
    "#;

    let detected = detect_malicious_clsid(html);
    println!("Detected: {:?}", detected); // ["IE ActiveX"]
}
```

## Error Handling

```rust
use regex_dfa::{Regex, Error};

match Regex::new(pattern) {
    Ok(re) => {
        // Use the regex
        if re.is_match(text) {
            println!("Match found!");
        }
    }
    Err(Error::RegexSyntax(e)) => {
        eprintln!("Invalid regex syntax: {}", e);
    }
    Err(Error::TooManyStates) => {
        eprintln!("Pattern too complex - use new_bounded()");

        // Retry with bounded compilation
        if let Ok(re) = Regex::new_bounded(pattern, 100000) {
            println!("Compiled with state limit");
        }
    }
    Err(Error::InvalidEngine(msg)) => {
        eprintln!("Engine error: {}", msg);
    }
}
```

## Performance Tips

1. **Compile once, use many times**: DFA compilation is expensive, but matching is very fast
   ```rust
   // ❌ Bad: Compiling in a loop
   for text in inputs {
       let re = Regex::new(pattern)?; // Slow!
       re.is_match(text);
   }

   // ✅ Good: Compile once
   let re = Regex::new(pattern)?;
   for text in inputs {
       re.is_match(text); // Fast!
   }
   ```

2. **Use bounded compilation for unknown patterns**: Prevents memory exhaustion
   ```rust
   let max_states = 100_000;
   let re = Regex::new_bounded(user_pattern, max_states)?;
   ```

3. **Simplify patterns when possible**: Fewer alternations = fewer states
   ```rust
   // Instead of: r"(a|b|c|d|e|f|g|h|i|j|k|l|m|n|o|p)"
   // Use: r"[a-p]"
   ```

## Limitations

- **No capture groups**: This crate doesn't support capturing subgroups
- **No backreferences**: Patterns like `\1` are not supported
- **Memory usage**: DFAs can be large (megabytes for complex patterns)
- **Unicode**: Full unicode support, but unicode character classes create many states

## Running the Examples

```bash
# Run the Suricata patterns example
cargo run --example suricata_patterns

# Build your own example
cargo new --bin my_dfa_app
cd my_dfa_app
# Add to Cargo.toml: regex_dfa = { path = "../regex-dfa" }
```

## Converting Your Suricata Patterns

For your specific patterns:

```rust
// Pattern 1: Extract just the CLSID for matching
// Original: <OBJECT\s+[^>]*classid\s*=\s*[\x22\x27]?\s*clsid\s*\x3a\s*\x7B?\s*66757BFC...
// Simplified for DFA:
let clsid = r"66757BFC-DA0C-41E6-B3FE-B6D461223FF5";
let re = Regex::new(clsid)?;

// Pattern 2: With offer status
// Original: ...E2883E8F-472F-4fb0-9522-AC9BF37916A7.+offer-(ineligible|preinstalled|declined|accepted)
// Split into two checks:
let clsid_re = Regex::new(r"E2883E8F-472F-4fb0-9522-AC9BF37916A7")?;
let offer_re = Regex::new(r"offer-(ineligible|preinstalled|declined|accepted)")?;
if clsid_re.is_match(html) && offer_re.is_match(html) {
    // Both patterns found
}

// Pattern 7: Simple keywords
let keywords_re = Regex::new(r"(SnapshotPath|CompressedPath|PrintSnapshot)")?;
```

## Further Reading

- See `examples/suricata_patterns.rs` for complete working examples
- Check the [original documentation](http://jneem.github.io/regex-dfa) for algorithm details
- Compare with Rust's standard `regex` crate for when to use each
