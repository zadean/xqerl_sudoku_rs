# xqerl_sudoku_rs - Rust Implementation with Tokio

A complete rewrite of the XQuery/Erlang sudoku solver into pure Rust with async/concurrent puzzle generation using Tokio.

## Architecture

This Rust implementation replaces the Erlang/OTP + XQuery 3.1 hybrid architecture with a pure Rust async server. All functionality is preserved, but implemented idiomatically in Rust.

```
┌──────────────────────────────────────────────────┐
│     Tokio Async Runtime                          │
│  (Replaces Erlang/OTP Supervisor + GenServer)    │
│                                                  │
│  ┌────────────────────────────────────────────┐  │
│  │  HTTP Server (Axum)                        │  │
│  │  • POST /api/puzzles/create                │  │
│  │  • GET  /api/puzzles                       │  │
│  │  • GET  /api/puzzles/:id                   │  │
│  │  • GET  /api/stats                         │  │
│  │  • GET  /api/test                          │  │
│  └────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────┘
                     │
      ┌──────────────┼──────────────┐
      │              │              │
      ▼              ▼              ▼
┌───────────┐  ┌──────────┐  ┌─────────────┐
│ Board     │  │ Solver   │  │ Generator   │
│ Data      │  │ All 14   │  │ Random + 3- │
│ Structure │  │ Solving  │  │ Phase Puzzle│
│           │  │ Algos    │  │ Creation    │
└───────────┘  └──────────┘  └─────────────┘
      │              │              │
      └──────────────┼──────────────┘
                     │
                     ▼
            ┌──────────────────┐
            │ In-Memory Store  │
            │ (RwLock<Vec>)    │
            └──────────────────┘
```

## Module Structure

### `board.rs` - Board Data Model
Equivalent to `sudoku-boards.xqm` (XQuery)

- **`Cell` struct**: Individual cell with value (0-9) and possible candidates
- **`Board` struct**: 9x9 grid using array-of-arrays
  - `from_string()` - Parse 81-digit strings
  - `to_string()` - Serialize to 81-digit string
  - `get_visible_cells()` - Sudoku visibility (row, column, 3x3 box)
  - `set_cell()` - Set value with constraint propagation
  - `get_unsolved()` - Find empty cells
  - `is_valid()` / `is_solved()` - Board state checks
  - `deep_clone()` - Independent copy

**Key Differences from XQuery**:
- Uses Rust's type system instead of XQuery maps/arrays
- Direct array indexing instead of nested map access
- Mutable references for efficiency instead of functional immutability

### `solver.rs` - Solving Algorithms
Equivalent to `sudoku-solve.xqm` (XQuery)

Implements 14 sudoku solving techniques organized by difficulty:

**SIMPLE (Weight=1):**
- Sole Candidate: Cell with 1 possible value
- Unique Candidate: Value appears in 1 cell of unit
- Block-Row/Column Interactions: Eliminate outside block

**MEDIUM (Weight=10):**
- Naked Pairs/Triples: N cells with N candidates
- Hidden Pairs/Subsets: N values in N cells

**ADVANCED (Weight=100+):**
- XY-Wing, XYZ-Wing, WXYZ-Wing: Hinge patterns
- X-Wing, X-Cycle: Rectangle/cycle patterns
- Singles Chain, XY-Chain: Alternating chains

**Key Components**:
```rust
pub struct ScoreMap {
    // Counters for each technique
    sole_candidate: u32,
    unique_candidate: u32,
    // ... etc
}

pub enum SolveStatus { Solved, Unsolved, Invalid }

pub struct SolveResult {
    pub board: Board,
    pub score: ScoreMap,
    pub status: SolveStatus,
}

impl Solver {
    pub fn solve(board: &Board) -> SolveResult  // Full solver
    pub fn solve_simple(board: &Board) -> SolveResult
    pub fn solve_medium(board: &Board) -> SolveResult
}
```

**Difficulty Classification** (from score weights):
- Easy: Total weight 0-5
- Medium: Total weight 6-30
- Hard: Total weight 31-100
- Expert: Total weight 100+

### `generator.rs` - Puzzle Generation
Equivalent to `sudoku-create.xqm` (XQuery)

- **`random_solved_board()`** - Generate random valid 81-digit solution
- **`random_solved_board_seeded(seed: u64)`** - Reproducible generation
- **`fill_board_random()`** - Recursive backtracking with random candidates
- **`create_puzzle(solved: &Board)`** - 3-phase hint removal:
  - Phase 1: Remove 4 cells (quad symmetry) if solvable with SIMPLE
  - Phase 2: Remove 2 cells (diagonal) if solvable with MEDIUM
  - Phase 3: Remove 1 cell if solvable with full solver
- **`board_to_id()`** - Unique puzzle ID generation

Uses `rand` crate with `StdRng` for seeded reproducibility (replaces XQuery's `random:seeded-permutation()`)

### `storage.rs` - Puzzle Persistence
Replaces XML collection in `http://xqerl.org/sudoku/puzzles/run8/`

- **`StoredPuzzle`** struct: Complete puzzle record
  - id, solution_id, puzzle_hints, solved_board
  - score (ScoreMap), difficulty, hint_count, created_at
- **`PuzzleStore`** async struct: In-memory store with RwLock
  - Async methods: `store()`, `get()`, `get_all()`, `get_by_difficulty()`
  - `get_stats()` - Aggregated technique usage across all puzzles

**Future**: Can be extended to use SQLite/PostgreSQL with sqlx

### `main.rs` - HTTP Server
Replaces Erlang/OTP application structure

**Framework**: Axum (lightweight async HTTP server)

**Endpoints**:
```
GET  /health                              # Liveness check
POST /api/puzzles/create                  # {"count": N}
GET  /api/puzzles                         # List all
GET  /api/puzzles/:id                     # Get by ID
GET  /api/puzzles/difficulty/:difficulty  # Filter by difficulty
GET  /api/stats                           # Aggregated stats
GET  /api/test                            # Solver test
```

**Concurrency**:
- Puzzle generation runs on Tokio's blocking thread pool (`spawn_blocking`)
- HTTP handlers are fully async
- Each puzzle is generated independently in parallel

## Comparison: Erlang/XQuery → Rust

| Aspect | Erlang/XQuery | Rust |
|--------|---------------|------|
| **Runtime** | Erlang VM | Tokio async |
| **Data Structure** | Nested XQuery maps/arrays | Rust structs/arrays |
| **Solving** | Interpreted XQuery functions | Compiled Rust methods |
| **Concurrency** | Lightweight processes | Tokio tasks |
| **Storage** | XML collection | In-memory RwLock (upgradable to DB) |
| **API** | Erlang modules (OTP) | HTTP/REST (Axum) |
| **Performance** | Good (interpreted) | Better (compiled + LLVM) |
| **Startup** | Seconds | Milliseconds |
| **Memory** | Dynamic BEAM | Static Rust |

## Building

```bash
cd xqerl_sudoku_rs
cargo build --release
```

## Running

```bash
cargo run
# Server listens on http://127.0.0.1:3000
```

## Testing

Generate 3 puzzles:
```bash
curl -X POST http://127.0.0.1:3000/api/puzzles/create \
  -H "Content-Type: application/json" \
  -d '{"count": 3}'
```

Get all puzzles:
```bash
curl http://127.0.0.1:3000/api/puzzles
```

Get statistics:
```bash
curl http://127.0.0.1:3000/api/stats
```

Test solver on a hard puzzle:
```bash
curl http://127.0.0.1:3000/api/test
```

## Key Improvements Over Original

1. **Type Safety**: Rust's strong type system catches errors at compile-time (XQuery is dynamically typed)
2. **Performance**: Compiled native code vs interpreted XQuery
3. **Memory Safety**: No null pointer dereferences, guaranteed memory safety
4. **Concurrency**: Modern async/await instead of Erlang lightweight processes
5. **HTTP API**: RESTful instead of Erlang module calls
6. **Single Binary**: No external XQuery compiler dependency
7. **Easier Deployment**: Self-contained executable

## Compatibility Notes

- Algorithm accuracy is equivalent to XQuery implementation
- Puzzle difficulty classification matches original weighted system
- Seeded RNG produces different puzzles than XQuery (different PRNG implementation)
- JSON API replaces Erlang function calls

## Future Enhancements

1. **Persistent Storage**: Integrate sqlx for SQLite/PostgreSQL
2. **Web UI**: Add static file serving for puzzle display
3. **Advanced Algorithms**: Add remaining techniques (Coloring, Forcing Chains, etc.)
4. **CLI Tool**: Add clap-based CLI for standalone puzzle generation
5. **WebSocket**: Real-time puzzle solving progress
6. **Difficulty Tuning**: ML-based difficulty prediction
7. **Distributed**: Multi-server puzzle generation coordination

## Testing Puzzles from Original

The original `test.xq` contained several hard puzzles testing advanced techniques. These can be ported as:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_xy_chain_puzzle() {
        let puzzle = "4.3.....8.8.3.34.2.2.8.4.91.1.6.9.6.4.1.....6.8.3.7.1....9.8.5.6....7.7.1...";
        let board = Board::from_string(puzzle).unwrap();
        let result = Solver::solve(&board);
        assert_eq!(result.status, SolveStatus::Solved);
        assert!(result.score.xy_chain > 0);
    }
}
```

## License

Same as original project (from xqerl_demo_sudoku)
