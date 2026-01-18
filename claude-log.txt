# Research Notebook: Understanding regex-dfa for TFHE Adaptation

Date: 2026-01-18
Goal: Understand the compilation pipeline (RegEx → NFA → DFA → executable graph) to modify it for TFHE programs.

---

## Session 1: Understanding the Compilation Pipeline

### Overview

The regex-dfa library compiles regular expressions eagerly into DFAs (unlike Rust's `regex` crate which uses lazy compilation). The full pipeline:

```
Regex string
    ↓ Nfa::from_regex()          [src/nfa/has_looks.rs:165]
NFA<u32, HasLooks>
    ↓ nfa.remove_looks()         [src/nfa/has_looks.rs:193]
NFA<u32, NoLooks>
    ↓ nfa.byte_me()              [src/nfa/no_looks.rs:248]
NFA<u8, NoLooks>
    ↓ nfa.determinize()          [src/dfa/mod.rs]
DFA<Ret>
    ↓ dfa.optimize()             [src/dfa/minimizer.rs]
DFA (minimized)
    ↓ dfa.compile()              [src/runner/program.rs]
TableInsts (executable bytecode)
```

All compilation happens during `Regex::new()`, not during matching.

---

### NFA Internal Representation

The NFA is stored as:

```rust
struct Nfa<Tok, Variant> {
    states: Vec<State<Tok>>,           // All states
    init: Vec<(Look, StateIdx)>,       // Initial states (for NoLooks variant)
    phantom: PhantomData<Variant>,
}

struct State<Tok> {
    accept: Accept,                     // Never, AtEoi, or Always
    consuming: RangeMultiMap<Tok, StateIdx>,  // Input-consuming transitions
    looking: Vec<LookPair>,             // ε-transitions with lookaround
    // ... other fields for lookahead bookkeeping
}
```

Key data structures:
- `RangeMultiMap<Tok, StateIdx>`: Maps character ranges to target states. "Multi" means one range can map to multiple targets (non-determinism).
- `LookPair`: ε-transition guarded by lookaround constraints (behind, ahead, target_state).

---

### ε-Transitions (Epsilon Transitions)

An ε-transition moves between states WITHOUT consuming input. Used for:

1. **Concatenation** (`ab`): Chain states with ε
2. **Alternation** (`a|b`): Fan out with ε to try both paths
3. **Repetition** (`a*`, `a+`, `a?`): Loops and skips via ε

Example for `a?b`:
```
        ε (skip 'a')
      +-------------+
      |             |
      v             |
[0] --a--> [1] --ε--+--> [2] --b--> [3 ✓]
```

DFAs cannot have ε-transitions — `remove_looks()` eliminates them.

---

### Accept States

```rust
enum Accept {
    Never,   // Not an accepting state
    AtEoi,   // Accept ONLY at End Of Input (for $, \b at end)
    Always,  // Unconditionally accept
}
```

Example:
- Pattern `a` → State is `Always` (match 'a' anywhere)
- Pattern `a$` → State is `AtEoi` (match 'a' only at end of input)

---

### Look Types (Lookaround Constraints)

```rust
enum Look {
    Full,        // Any character (no constraint)
    WordChar,    // Matches \w (word characters)
    NotWordChar, // Matches \W (non-word characters)
    NewLine,     // Matches \n only
    Boundary,    // Start/end of input (empty set)
    Empty,       // Impossible (intersection failure)
}
```

Used in `LookPair` for ε-transitions:
```rust
LookPair {
    behind: Look,   // What character must be BEFORE current position
    ahead: Look,    // What character must be AFTER current position
    target_state: StateIdx,
}
```

Example: `\b` (word boundary) uses:
- `(WordChar, NotWordChar)` — word char behind, non-word ahead
- `(NotWordChar, WordChar)` — non-word behind, word char ahead

---

### Step 3: byte_me() — Unicode to UTF-8 Conversion

Converts NFA<u32, NoLooks> (Unicode codepoints) to NFA<u8, NoLooks> (UTF-8 bytes).

Problem: Strings are UTF-8 encoded with variable-length byte sequences:
- 'a' (97) → [0x61] (1 byte)
- '中' (20013) → [0xE4, 0xB8, 0xAD] (3 bytes)

A single Unicode transition expands into a chain of byte transitions:

Before: `State 0 --'中'--> State 1`
After:  `State 0 --0xE4--> State 2 --0xB8--> State 3 --0xAD--> State 1`

---

### NFA Output Format (Debug)

Modified the Debug implementation to show characters:

```
State 0 (Never):
    Consuming:
        99 ('c') => 1        // Single char transition
        97 ('a') -- 122 ('z') => 2   // Range transition
    Looking:
        (Full,Full) => 3     // ε-transition
        (WordChar,NotWordChar) => 4  // Word boundary
State 1 (Always):
    look Full, tokens 0, state 1    // Accepting state
```

---

### Visualization Tool

Added `--nfa` flag to `examples/dfa_internals.rs`:

```bash
cargo run --example dfa_internals -- "a?b" --nfa     # Show only NFA stages
cargo run --example dfa_internals -- "(cat|dog)"     # Show full pipeline
cargo run --example dfa_internals -- '\bword\b'      # Word boundaries
```

---

## Open Question: Multi-Character Matching

**Q: Can a single state match multiple characters?**

In classical automata: No. Each transition consumes exactly ONE symbol.

To match "hello", you need 5 transitions: `S0 --h--> S1 --e--> S2 --l--> S3 --l--> S4 --o--> S5`

**For TFHE context**, multi-character matching could reduce expensive operations:
- Chunk-based: Treat 2-4 bytes as a "super-symbol" (table grows to 256^n)
- Different encoding: Parallel character checks
- Precomputed tables: Multi-character match results

Trade-off: Fewer transitions vs. larger transition tables and harder prefix sharing.

---

## Files Modified

1. `notes.md` — Created development notes
2. `examples/dfa_internals.rs` — Added `--nfa` flag for NFA-only output
3. `src/nfa/mod.rs` — Improved Debug output to show characters (e.g., `99 ('c')`)

---

## Session 2: Pattern Compatibility Testing

### PCRE Pattern Compatibility

Tested 7,772 patterns from Suricata IDS rules against regex-dfa.

| Metric | Count | Percentage |
|--------|-------|------------|
| Successful | 6,041 | 77.7% |
| Failed | 1,731 | 22.3% |

**Main failure categories:**
1. State overflow (267 patterns) — DFA exceeded 10,000 states
2. Unsupported escapes: `\/`, `\:`, `\-` (~800 patterns) — easy to fix with preprocessing
3. Lookahead/lookbehind: `(?!...)`, `(?=...)`, `(?<=...)` — not supported in DFA
4. Named groups/backreferences: `(?P<name>...)`, `(?P=name)` — cannot be expressed in DFA

See `compatibility.md` for full report.

---

### State Explosion Analysis

Two explosion points identified:

#### Type 1: `byte_me()` explosion — Unicode → UTF-8 conversion

| Pattern | NFA (codepoints) | NFA (bytes) | Reason |
|---------|------------------|-------------|--------|
| `[a-zA-Z0-9_]` | 2 | 2 | ASCII only |
| `\w` | 2 | **736** | Unicode letters from all scripts |
| `\w{8}` | 40 | **>10,000** | 736 × 8+ intermediate states |

**Why**: `\w` matches Unicode word characters (Arabic, Chinese, Cyrillic, etc.), each requiring multi-byte UTF-8 state chains.

**Example**:
```
Pattern: ^ID=\w{8}-\w{4}-\w{4}-\w{4}-\w{12}
NFA (codepoints): 40 states
byte_me(): EXPLODED (>10,000 states)
```

#### Type 2: `determinize()` explosion — NFA → DFA subset construction

```
Pattern: ^.{0,100}\+0A.{0,100}\x40

Step 2 (NFA codepoints): 205 states
Step 3 (NFA bytes):      7,369 states
Step 4 (DFA):            EXPLODED (>10,000)
```

**Why**: `.{0,100}` means the NFA can be in any of 101 positions simultaneously. Two such patterns means up to 101 × 101 = 10,201 possible DFA states (subset construction creates a state for each unique combination of NFA states).

---

### Key Insight for TFHE

If targeting ASCII-only inputs (network traffic, logs), replace Unicode classes:

| Original | ASCII Replacement | State Reduction |
|----------|-------------------|-----------------|
| `\w` | `[a-zA-Z0-9_]` | 736 → 2 states |
| `\s` | `[ \t\r\n]` | ~100 → 4 states |
| `.` | `[\x00-\x7f]` | ~1000 → 128 states |

This dramatically reduces state counts and avoids byte_me() explosions.

---

## Tools Created

1. `examples/check_patterns.rs` — Check pattern compatibility against regex-dfa
   ```bash
   cargo run --example check_patterns -- /path/to/patterns.txt --max-states 10000
   ```

2. `examples/find_explosions.rs` — Find patterns that cause state explosion
   ```bash
   cargo run --example find_explosions -- /path/to/patterns.txt
   ```

---

## Files Created/Modified

1. `notes.md` — Development notes
2. `compatibility.md` — Full PCRE compatibility report
3. `claude-log.txt` — This research notebook
4. `examples/dfa_internals.rs` — Added `--nfa` flag
5. `examples/check_patterns.rs` — Pattern compatibility checker
6. `examples/find_explosions.rs` — State explosion analyzer
7. `src/nfa/mod.rs` — Improved Debug output to show characters

---

## Next Steps

- [ ] Understand `determinize()` — subset construction algorithm
- [ ] Examine final `TableInsts` structure
- [ ] Investigate modifications needed for TFHE compatibility
- [ ] Explore multi-character transition approaches for FHE efficiency
- [ ] Consider ASCII-only mode to avoid Unicode state explosion
