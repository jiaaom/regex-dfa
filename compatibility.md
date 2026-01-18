# Pattern Compatibility Report

## Test Configuration

- **Source file**: `/Users/mason/Developer/Projects/Firewall/PIDS/pcre.txt`
- **Patterns source**: Suricata IDS rules (PCRE patterns)
- **Max DFA states**: 10,000
- **Test command**: `cargo run --release --example check_patterns -- /path/to/pcre.txt`

## Summary

| Metric | Count | Percentage |
|--------|-------|------------|
| **Total patterns** | 7,772 | 100% |
| **Successful** | 6,041 | 77.7% |
| **Failed** | 1,731 | 22.3% |
| **Skipped (comments/empty)** | 9 | — |

## Error Categories

### 1. State Overflow (267 patterns)

The DFA exceeded 10,000 states during compilation. These patterns cause state explosion due to:
- Unbounded repetition with wildcards (e.g., `.{0,100}`)
- Complex alternations
- Large character classes combined with repetition

**Examples**:
```
ElseIf\s+\x28\x24\w{8,10}\s+\x2deq\s+\x27\w{2}\x27\x29\s+\x7...
^.{0,100}\+0A.{0,100}\x40
^\d+\s*[^\r\n]{50,}
```

**Mitigation**: Increase `max_states` parameter or simplify patterns.

---

### 2. Unsupported Escape: `\/` (~700+ patterns)

PCRE allows escaping forward slash as `\/`, but this regex engine doesn't recognize it. Forward slash has no special meaning in regex and doesn't need escaping.

**Examples**:
```
^\/[a-z]{2}\x3Fv\x3D[0-9]$
^\/[a-z]{15}[0-9]\.php$
^\/(?:[A-Za-z]+\d?\/)?\?q=...
```

**Mitigation**: Preprocess patterns to replace `\/` with `/`.

---

### 3. Unsupported Escape: `\:` (72 patterns)

Same issue as `\/` — colon has no special meaning and doesn't need escaping.

**Examples**:
```
^\s*(?:ftps?|https?|php)\:\/
config\[installdir\]=\s*(?:ftps?|https?|php)\:\/
```

**Mitigation**: Preprocess patterns to replace `\:` with `:`.

---

### 4. Negative Lookahead `(?!...)` (many patterns)

PCRE negative lookahead is not supported. This is a zero-width assertion that matches if the pattern inside does NOT match at the current position.

**Examples**:
```
CN=(?:[^\r\n]+?\.)?dns\.lavate\.ch(?!\.)
\x2F(?!Subtype)(S|#53)(u|#75)...
(?P<a>(?!(?P=v))[0-9a-z]{2})
```

**Mitigation**: None — these patterns cannot be directly converted to DFA. Would need pattern redesign or alternative matching strategy.

---

### 5. Positive Lookahead `(?=...)` (some patterns)

PCRE positive lookahead is not supported. This is a zero-width assertion that matches if the pattern inside DOES match at the current position.

**Examples**:
```
(?=.*?[?&]oq=(?=[A-Za-z_-]*[0-9])...
```

**Mitigation**: None — same as negative lookahead.

---

### 6. Lookbehind `(?<=...)` and `(?<!...)` (some patterns)

PCRE lookbehind assertions are not supported.

**Examples**:
```
(?<=(?:\?|&))pasa=(?!&).
```

**Mitigation**: None — same as lookahead.

---

### 7. Named Capture Groups `(?P<name>...)` (some patterns)

PCRE named capture groups are not supported.

**Examples**:
```
^(?P<addr1>.{4})(?P<addr2>.{4})...
^(?P<arrayName>[a-z]{1,50})\x20\x3d...
```

**Mitigation**: Replace with regular groups `(...)` if backreferences aren't needed.

---

### 8. Backreferences `(?P=name)` (some patterns)

PCRE backreferences to named groups are not supported. These require matching the same text that was captured by a previous group.

**Examples**:
```
(?P=addr2)(?P=addr1)
(?P=base_dir)\.js\?
```

**Mitigation**: None — backreferences cannot be expressed in a DFA. They require NFA or backtracking engine.

---

### 9. Unsupported Escape: `\-` (few patterns)

Hyphen doesn't need escaping outside character classes.

**Examples**:
```
^[0-9a-f]{8}-(?:([0-9a-f]{4})\-){3}[0-9a-f]{12}$
=[a-z0-9\(_~\-\.\x00]{300,}\x00$
```

**Mitigation**: Preprocess patterns to replace `\-` with `-` (outside character classes).

---

## Feature Support Matrix

| Feature | PCRE Syntax | Supported | Notes |
|---------|-------------|-----------|-------|
| Literals | `abc` | Yes | |
| Character classes | `[a-z]`, `[^0-9]` | Yes | |
| Wildcards | `.` | Yes | |
| Alternation | `a\|b` | Yes | |
| Quantifiers | `*`, `+`, `?`, `{n,m}` | Yes | |
| Anchors | `^`, `$` | Yes | |
| Word boundary | `\b`, `\B` | Yes | |
| Shorthand classes | `\d`, `\w`, `\s` | Yes | |
| Hex escapes | `\x00`, `\x7B` | Yes | |
| Non-capturing groups | `(?:...)` | Yes | |
| Case insensitive | `(?i)` | Yes | |
| Escaped forward slash | `\/` | **No** | Use `/` instead |
| Escaped colon | `\:` | **No** | Use `:` instead |
| Escaped hyphen | `\-` | **No** | Use `-` instead |
| Negative lookahead | `(?!...)` | **No** | Cannot be expressed in DFA |
| Positive lookahead | `(?=...)` | **No** | Cannot be expressed in DFA |
| Lookbehind | `(?<=...)`, `(?<!...)` | **No** | Cannot be expressed in DFA |
| Named groups | `(?P<name>...)` | **No** | |
| Backreferences | `(?P=name)`, `\1` | **No** | Cannot be expressed in DFA |
| Atomic groups | `(?>...)` | **No** | |
| Possessive quantifiers | `*+`, `++` | **No** | |
| Conditionals | `(?(cond)yes\|no)` | **No** | |
| Recursion | `(?R)`, `(?1)` | **No** | |

---

## Recommendations for TFHE Adaptation

### 1. Preprocessing Pipeline

Create a preprocessing step to fix simple escape issues:

```python
def preprocess_pattern(pattern):
    # Remove unnecessary escapes
    pattern = pattern.replace(r'\/', '/')
    pattern = pattern.replace(r'\:', ':')
    # Handle \- outside character classes (more complex)
    return pattern
```

### 2. Pattern Filtering

Filter out patterns using unsupported features before compilation:

```python
unsupported_patterns = [
    r'\(\?!',      # Negative lookahead
    r'\(\?=',      # Positive lookahead
    r'\(\?<[!=]',  # Lookbehind
    r'\(\?P<',     # Named groups
    r'\(\?P=',     # Backreferences
]
```

### 3. State Limit Tuning

For patterns that fail with state overflow:
- Increase `max_states` if memory allows
- Consider pattern simplification
- Use lazy DFA (standard `regex` crate) as fallback

### 4. Compatibility Rate

With preprocessing to fix escape issues, expected compatibility:
- Current: **77.7%**
- After escape fixes: **~85-90%** (estimated)
- Remaining failures: Lookahead/backreference patterns + state overflow

---

## Test Tool

The `check_patterns` example can be used to test pattern files:

```bash
# Test with default 10,000 state limit
cargo run --example check_patterns -- /path/to/patterns.txt

# Test with custom state limit
cargo run --example check_patterns -- /path/to/patterns.txt --max-states 50000
```
