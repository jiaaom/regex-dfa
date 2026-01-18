# Development Notes

## Current Goal

Understand the compilation pipeline (RegEx → NFA → DFA → executable graph) so I can modify it to fit the needs for TFHE (Fully Homomorphic Encryption over Torus) programs.

## Pipeline Overview

```
Regex string
    ↓ Nfa::from_regex()          [src/nfa/has_looks.rs:165]
NFA<u32, HasLooks>
    ↓ nfa.remove_looks()         [src/nfa/has_looks.rs:193]
NFA<u32, NoLooks>
    ↓ nfa.byte_me()              [src/nfa/no_looks.rs]
NFA<u8, NoLooks>
    ↓ nfa.determinize()          [src/dfa/mod.rs]
DFA<Ret>
    ↓ dfa.optimize()             [src/dfa/minimizer.rs]
DFA (minimized)
    ↓ dfa.compile()              [src/runner/program.rs]
TableInsts (executable bytecode)
```

## Key Data Structures

- `NFA<Tok, Variant>`: States stored in `Vec<State<Tok>>`, transitions in `RangeMultiMap<Tok, StateIdx>`
- `State<Tok>`: Contains `consuming` (input transitions) and `looking` (ε-transitions with lookaround)
- `DFA<Ret>`: Deterministic version, single transition per input

## Questions / Areas to Explore

- [ ] How does `determinize()` work? (subset construction)
- [ ] What does the final `TableInsts` structure look like?
- [ ] What modifications are needed for FHE compatibility?

## Session Log

- Learned about NFA internal representation (`RangeMultiMap`, `State`, `LookPair`)
- Learned about ε-transitions and why NFA is built first (natural mapping from regex, O(n) states)
- Identified that NFA→DFA happens in `Regex::new()` via `determinize()`

## Example RegEx Patterns

I may use these two patterns for now:

- ^(?:curl|wget)  
- \.(?:exe|dll|ini)$

Here’s what they mean in plain English:

1) ^(?:curl|wget)
- ^ means “start of the string.”
- (?:curl|wget) means “either the word curl or wget.”
So this matches strings that start with curl or wget.

2) \.(?:exe|dll|ini)$
- \. means a literal dot . (not “any character”).
- (?:exe|dll|ini) means “either exe, dll, or ini.”
- $ means “end of the string.”
So this matches strings that end with .exe, .dll, or .ini.

## How to Visualize DFA Compilation Pipeline:

```sh
cargo run --example dfa_internals -- "^(?:curl|wget)"
```
