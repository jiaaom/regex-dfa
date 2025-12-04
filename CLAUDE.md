# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`regex_dfa` is a Rust crate that compiles regular expressions into deterministic finite automata (DFAs). Unlike lazy DFA compilation (as in Rust's `regex` crate), this library performs eager compilation, trading increased memory usage and compilation time for faster matching performance.

**Status**: This codebase has been successfully ported from Rust 2015 (circa 2016) to Rust 2021 and modern stable Rust. Key changes in the port:
- Vendored the `refinery` crate's `Partition` implementation into src/partition.rs (modern refinery is a different project)
- Updated to Rust 2021 edition
- Replaced `try!` macros with `?` operator
- Added `dyn` keyword to trait objects
- Updated module paths to use `crate::` prefix
- Updated dependencies (lazy_static, memchr, num-traits, utf8-ranges)
- Disabled nightly-only features (benchmarking with `test` crate)
- All 45 tests passing

## Build and Test Commands

```bash
# Build the project
cargo build

# Run all tests (requires nightly Rust)
cargo test

# Run only library tests
cargo test --lib

# Run specific test file
cargo test --test examples

# Run benchmarks
cargo bench

# Build documentation
cargo doc --open
```

## High-Level Architecture

### Compilation Pipeline

Regular expressions go through a multi-stage transformation pipeline:

1. **Regex string → NFA<u32, HasLooks>** (src/nfa/has_looks.rs)
   - Parse regex syntax via `regex-syntax` crate
   - Build NFA with lookaround assertions
   - Initial state is implicitly state 0

2. **NFA<u32, HasLooks> → NFA<u32, NoLooks>** (src/nfa/no_looks.rs)
   - Remove lookaround assertions by creating explicit initial states
   - The `remove_looks()` algorithm (documented in src/nfa/no_looks.rs) handles word boundaries (`\b`, `\B`) and anchors (`^`, `$`)
   - Multiple initial states in `init` field based on lookaround context

3. **NFA<u32, NoLooks> → NFA<u8, NoLooks>** (src/nfa/no_looks.rs)
   - Convert from Unicode codepoint transitions to UTF-8 byte transitions
   - Called via `byte_me()`
   - May create additional states to handle multi-byte UTF-8 sequences

4. **NFA<u8, NoLooks> → DFA<Ret>** (src/dfa/mod.rs)
   - Determinization via `determinize()` or `determinize_longest()`
   - Subset construction algorithm
   - Can fail if too many states would be created (controlled by `max_states` parameter)

5. **DFA optimization** (src/dfa/minimizer.rs)
   - Minimize states using Hopcroft's algorithm
   - Sort states in DFS order for better cache locality
   - Extract common prefixes for optimization

6. **DFA → TableInsts** (src/runner/program.rs)
   - Compile to executable bytecode table
   - Use byte equivalence classes to reduce table size
   - Final executable form run by Engine implementations

### Engine Types

The final `Regex` uses one of three execution engines (src/regex.rs):

- **EmptyEngine**: Matches nothing (for empty/impossible regexes)
- **AnchoredEngine** (src/runner/anchored.rs): For anchored patterns (`^...`)
- **ForwardBackwardEngine** (src/runner/forward_backward.rs): For general patterns
  - Forward DFA: anchored version for finding match endpoints
  - Backward DFA: runs in reverse to find match start points
  - Prefix optimization: extracts common prefixes for fast scanning

### The Look System

The `Look` enum (src/look.rs) represents lookaround context:
- `Full`: matches any character
- `WordChar`: matches word characters (as in `\w`)
- `NotWordChar`: matches non-word characters
- `NewLine`: matches `\n`
- `Boundary`: special marker for word boundaries and anchors
- `Empty`: matches nothing (intersection failure)

Looks form a partial ordering and support intersection operations, critical for the `remove_looks()` algorithm.

### Type Parameters

**NFA<Tok, Variant>**:
- `Tok`: Token type (`u32` for codepoints, `u8` for bytes)
- `Variant`: Either `HasLooks` or `NoLooks`, determining representation and available operations

**DFA<Ret>**:
- `Ret`: Return value type when a match is found (typically `(Look, u8)` carrying lookaround info and byte count)

## Code Organization

- **src/lib.rs**: Crate entry point and documentation
- **src/regex.rs**: Public `Regex` API and engine selection logic
- **src/nfa/**: NFA implementation split between HasLooks and NoLooks variants
- **src/dfa/**: DFA construction, minimization, and prefix extraction
- **src/runner/**: Execution engines (anchored and forward-backward)
- **src/look.rs**: Lookaround context system
- **src/graph.rs**: Graph utilities (DFS, SCC, etc.)
- **src/unicode.rs**: Unicode character class data
- **src/error.rs**: Error types

## Memory Management

The crate provides `Regex::new_bounded(re, max_states)` to limit memory consumption. DFA construction will fail with an error if it would exceed the state limit. This is critical because:
- Unicode character classes can cause state explosion
- Eager DFA construction creates all possible states upfront
- Some patterns may require megabytes of memory as DFAs

## Testing Approach

- **tests/matches.rs**: Comprehensive regex matching tests (auto-generated from regex test suite)
- **Inline tests**: Module-level tests throughout src/ for individual algorithms
- **Benchmarks**: benches/ directory contains performance tests comparing to standard `regex` crate
- Use `#[cfg(test)]` blocks for test-specific code and helper functions
