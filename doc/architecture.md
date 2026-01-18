# Architecture

This document describes the high-level architecture of `regex-dfa`, including the compilation pipeline, key types, and execution engines.

## Overview

The library transforms regular expressions through a multi-stage compilation pipeline, ultimately producing executable bytecode that runs in a table-driven DFA engine.

```
┌─────────────────┐
│  Regex String   │
└────────┬────────┘
         │ parse (regex-syntax)
         ▼
┌─────────────────┐
│ NFA<u32,        │  Unicode codepoints with
│   HasLooks>     │  lookaround transitions
└────────┬────────┘
         │ remove_looks()
         ▼
┌─────────────────┐
│ NFA<u32,        │  Unicode codepoints with
│   NoLooks>      │  explicit initial states
└────────┬────────┘
         │ byte_me()
         ▼
┌─────────────────┐
│ NFA<u8,         │  UTF-8 bytes with
│   NoLooks>      │  multi-byte sequences
└────────┬────────┘
         │ determinize()
         ▼
┌─────────────────┐
│ DFA<(Look, u8)> │  Deterministic automaton
└────────┬────────┘
         │ optimize()
         ▼
┌─────────────────┐
│ DFA (minimized) │  Minimal DFA
└────────┬────────┘
         │ compile()
         ▼
┌─────────────────┐
│   TableInsts    │  Executable bytecode
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│     Engine      │  AnchoredEngine or
│                 │  ForwardBackwardEngine
└─────────────────┘
```

## Source File Organization

```
src/
├── lib.rs                  Crate entry point
├── regex.rs                Public Regex API
├── error.rs                Error types
├── look.rs                 Look enum (lookaround context)
├── graph.rs                Graph utilities (DFS, SCC)
├── partition.rs            Partition data structure (for minimization)
├── unicode.rs              Unicode character class data
│
├── nfa/
│   ├── mod.rs              NFA struct and common operations
│   ├── has_looks.rs        NFA with lookaround assertions
│   └── no_looks.rs         NFA without lookarounds, determinization
│
├── dfa/
│   ├── mod.rs              DFA struct and operations
│   ├── minimizer.rs        Hopcroft's minimization algorithm
│   ├── prefix_searcher.rs  Prefix extraction for optimization
│   └── trie.rs             Trie for prefix handling
│
└── runner/
    ├── mod.rs              Engine trait definition
    ├── program.rs          TableInsts bytecode
    ├── anchored.rs         AnchoredEngine implementation
    └── forward_backward.rs ForwardBackwardEngine implementation
```

## Compilation Pipeline Stages

### Stage 1: Parsing to NFA with Looks

**File**: `src/nfa/has_looks.rs`

**Function**: `Nfa::from_regex(re: &str) -> Result<Nfa<u32, HasLooks>>`

The input regex string is parsed using the `regex-syntax` crate, then converted to an NFA. At this stage:

- Tokens are Unicode codepoints (`u32`)
- Lookaround assertions are represented as `Looking` transitions (epsilon transitions with look predicates)
- The initial state is implicitly state 0
- States have only `Always` or `Never` acceptance

**Looking transitions** allow the NFA to move between states without consuming input, but only when the current lookaround context matches the predicate.

### Stage 2: Removing Lookarounds

**File**: `src/nfa/has_looks.rs`

**Function**: `remove_looks(self) -> Nfa<u32, NoLooks>`

This complex transformation eliminates lookaround assertions by:

1. Computing epsilon-closures that preserve look predicates
2. Creating explicit initial states for different lookaround contexts
3. Converting `Looking` transitions to conditional `Consuming` transitions
4. Adding states for lookahead conditions that restrict transitions

After this stage:
- Multiple initial states in the `init` field, each tagged with required context
- States can have `AtEoi` acceptance (accepts only at end of input)
- No more epsilon transitions

See [Algorithms: remove_looks](./algorithms.md#remove_looks-algorithm) for details.

### Stage 3: Unicode to UTF-8 Bytes

**File**: `src/nfa/no_looks.rs`

**Function**: `byte_me(self, max_states: usize) -> Result<Nfa<u8, NoLooks>>`

Converts the NFA from Unicode codepoints to UTF-8 bytes:

- Uses `utf8_ranges` crate to generate UTF-8 byte sequences
- Merges sequences with common prefixes to reduce state explosion
- Creates additional states for multi-byte UTF-8 sequences (up to 4 bytes per character)

This stage may fail if it would create more states than `max_states`.

### Stage 4: NFA Transformations

For general (non-anchored) patterns, additional transformations are needed:

**Anchoring**: `anchor(max_states) -> Result<Nfa<u8, NoLooks>>`
- Adds self-loop transitions to make the regex anchored to start
- Used for the forward pass of ForwardBackwardEngine

**Reversal**: `reverse(max_states) -> Result<Nfa<u8, NoLooks>>`
- Reverses all transitions
- Initial states become accepting, accepting states become initial
- Used for the backward pass

### Stage 5: Determinization

**File**: `src/nfa/no_looks.rs`

**Functions**:
- `determinize(max_states) -> Result<Dfa<(Look, u8)>>`
- `determinize_longest(max_states) -> Result<Dfa<(Look, u8)>>`

Converts the NFA to a DFA using the subset construction algorithm:

1. Build powerset of NFA states
2. Compute transitions for each subset
3. Track accepting states and their return values
4. Optionally prefer longest matches with `determinize_longest`

Returns a DFA where the return value is `(Look, u8)`:
- `Look`: Lookaround context information
- `u8`: Byte count for lookahead

May fail if it would exceed `max_states`.

### Stage 6: DFA Optimization

**File**: `src/dfa/mod.rs`

**Function**: `optimize(self) -> Dfa<Ret>`

Optimizes the DFA through:

1. **Minimization**: Hopcroft's algorithm merges equivalent states
2. **Sorting**: DFS order for better cache locality
3. **Pruning**: Remove unreachable states

### Stage 7: Bytecode Compilation

**File**: `src/runner/program.rs`

**Function**: `compile(self) -> TableInsts<Ret>`

Compiles the DFA to executable bytecode:

- Creates byte equivalence classes to reduce table size
- Builds transition table indexed by `state << log_num_classes + class`
- Final form used by Engine implementations

## Key Types

### NFA<Tok, Variant>

The NFA type is parameterized by:

- `Tok`: Token type
  - `u32` for Unicode codepoints
  - `u8` for UTF-8 bytes

- `Variant`: Marker type indicating processing stage
  - `HasLooks`: Contains lookaround transitions
  - `NoLooks`: Lookarounds removed, ready for determinization

```rust
pub struct Nfa<Tok, Variant> {
    states: Vec<State<Tok>>,
    init: Vec<(Look, StateIdx)>,
    phantom: PhantomData<Variant>,
}
```

### DFA<Ret>

The DFA type is parameterized by return value type:

```rust
pub struct Dfa<Ret: 'static> {
    states: Vec<State<Ret>>,
    pub init: Vec<Option<StateIdx>>,  // One per Look variant
}

pub struct State<Ret> {
    pub transitions: RangeMap<u8, StateIdx>,
    pub accept: Accept,
    pub ret: Option<Ret>,
}
```

### Look

The `Look` enum represents lookaround context:

```rust
pub enum Look {
    Full,        // Matches any character
    WordChar,    // Matches \w
    NotWordChar, // Matches non-\w
    NewLine,     // Matches \n
    Boundary,    // Word boundary/anchor marker
    Empty,       // Matches nothing
}
```

Looks form a partial ordering and support intersection, critical for the lookaround removal algorithm.

### Accept

State acceptance modes:

```rust
pub enum Accept {
    Never,   // Non-accepting state
    AtEoi,   // Accepts only at end of input
    Always,  // Always accepting when reached
}
```

## Execution Engines

The final `Regex` uses one of three execution engines, selected automatically based on pattern properties.

### EmptyEngine

For regexes that match nothing (empty language). Always returns `None`.

### AnchoredEngine

**File**: `src/runner/anchored.rs`

For anchored patterns (those starting with `^`):

- Simple forward scan from position 0
- Table-driven state transitions
- Returns match immediately when accepting state reached

```rust
pub struct AnchoredEngine<Ret> {
    prog: TableInsts<Ret>,
}
```

### ForwardBackwardEngine

**File**: `src/runner/forward_backward.rs`

For general unanchored patterns:

```rust
pub struct ForwardBackwardEngine<Ret> {
    forward: TableInsts<(usize, u8)>,
    backward: TableInsts<Ret>,
    prefix: Prefix,
}
```

**Two-pass matching algorithm**:

1. **Prefix scan**: Use `memchr` to find potential match starts
2. **Forward pass**: Run anchored DFA to find match endpoint
3. **Backward pass**: Run reversed DFA from endpoint to find match start

**Prefix optimization**:

```rust
pub enum Prefix {
    Empty,
    ByteSet { bytes: Vec<bool>, offset: usize },
    Byte { byte: u8, offset: usize },
}
```

Extracts common prefixes from the forward DFA:
- Limits: max 30 prefixes, max 15 bytes per prefix
- Uses `memchr` for fast byte scanning
- Dramatically speeds up matching for patterns with common prefixes

## Engine Selection

Engine selection happens automatically in `Regex::new()`:

1. If the NFA is empty (matches nothing): `EmptyEngine`
2. If the pattern is anchored (`^...`): `AnchoredEngine`
3. Otherwise: `ForwardBackwardEngine`

The selection is based on the NFA's `is_anchored()` and `is_empty()` methods.
