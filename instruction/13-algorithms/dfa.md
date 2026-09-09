# DFA (Deterministic Finite Automata)

`13-algorithms/regex-engine.md` covers how a regex engine uses a DFA
underneath. This file covers the DFA itself — the general theory any
table-driven state machine (a protocol parser, a config lexer) rests on.

## What to learn

### States, transitions, one lookup per byte
A DFA is `(states, alphabet, transition_fn, start, accept_states)`, where
`transition_fn(state, byte) -> state` is total and deterministic: exactly
one next state for every `(state, byte)` pair. Matching becomes a loop —
no backtracking, no branching on multiple possibilities:

```rust
struct Dfa {
    table: Vec<[usize; 256]>, // table[state][byte] -> next state
    accept: Vec<bool>,
}

fn run(dfa: &Dfa, input: &[u8]) -> bool {
    let mut state = 0;
    for &b in input {
        state = dfa.table[state][b as usize];
    }
    dfa.accept[state]
}
```

This is why a DFA is the fastest matching structure available: O(n) with a
tiny, predictable constant — one array index per input byte, no recursion,
no allocation.

### Where a DFA comes from: subset construction
DFAs are rarely written by hand; they're derived from an NFA (built by
Thompson construction, see `13-algorithms/regex-engine.md`) via **subset
construction**: each DFA state is the *set* of NFA states reachable on
some input prefix. The number of subsets is exponential in the worst
case — this is exactly the memory blowup `regex-engine.md`'s lazy-DFA
section describes and solves by building states on demand; don't
duplicate that discussion, read it there.

### Minimization
Many DFA states are behaviorally identical — they accept exactly the same
set of future inputs — and can be merged without changing what the
automaton matches. Hopcroft's algorithm finds and merges these in
O(n log n), which matters for hand-built protocol-parsing DFAs (not
regex-derived ones) where a naive construction produces far more states
than necessary: fewer states means a smaller table and better cache
behavior on the hot path.

### The memory trade in a full transition table
The table above is `|states| × 256` entries — fast, but wasteful when most
states only care about a handful of distinct bytes (e.g. an HTTP method
matcher only branches on `G`, `P`, `D`, `H`, ...). A sparse
representation (a small match arm or a `HashMap<u8, usize>` per state)
trades one indirection for much less memory. Pick the dense table when
the DFA is small and hot (a handful of states checked on every byte of
every request); pick sparse when it's large and checked rarely.

## Practice
1. Hand-build the DFA (not the NFA) for matching one of the fixed set of
   HTTP methods (`GET`, `POST`, `PUT`, ...) as a table, and use it as the
   first byte-classification step in `labs/01-http-parser`.
2. Apply subset construction by hand to a small NFA for `a(b|c)*d` and
   confirm your resulting DFA table gives the same accept/reject verdicts
   as tracing the NFA state set (the exercise in `regex-engine.md`).
3. Find two behaviorally-equivalent states in a DFA you built and merge
   them manually; confirm the merged automaton still accepts the same
   language.
4. Compare a dense `[usize; 256]`-per-state table against a sparse
   `HashMap<u8, usize>` version on your method-matching DFA — measure
   both memory and per-byte lookup time.
