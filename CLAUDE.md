# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Sisyphus is a Sokoban puzzle solver written in Rust (edition 2024). It uses the IDA* (Iterative Deepening A*) search algorithm with Zobrist hashing-based transposition tables to find optimal solutions to Sokoban puzzles. The solver supports bidirectional search, multiple heuristics, and advanced pruning techniques including dead square detection and PI-corral pruning.

## Development Commands

### Building
```bash
cargo build           # Build the project
cargo build --release # Build with optimizations (recommended for solving)
```

### Running
The solver takes an XSB-format level file and one or more level numbers:
```bash
cargo run -- <FILE> <LEVEL> [LEVEL_END] [OPTIONS]
cargo run -- levels.xsb 1              # Solve level 1
cargo run -- levels.xsb 1 10           # Solve levels 1-10
cargo run -- levels.xsb 5 --print-solution  # Show step-by-step solution
cargo run -- levels.xsb 3 -n 10000000  # Set max nodes explored
cargo run -- levels.xsb 1 -H null      # Use null heuristic (pure iterative deepening)
cargo run -- levels.xsb 1 -H simple    # Use simple heuristic (admissible)
cargo run -- levels.xsb 1 -H greedy    # Use greedy heuristic (fast, not admissible)
cargo run -- levels.xsb 1 -H hungarian # Use Hungarian algorithm (optimal, default)
cargo run -- levels.xsb 1 -d forward   # Search forwards from initial state
cargo run -- levels.xsb 1 -d reverse   # Search backwards from goal state
cargo run -- levels.xsb 1 -d bidirectional  # Bidirectional search (default)
cargo run -- levels.xsb 1 --no-freeze-deadlocks  # Disable freeze deadlock detection
cargo run -- levels.xsb 1 --no-dead-squares      # Disable dead square pruning
cargo run -- levels.xsb 1 --no-pi-corrals        # Disable PI-corral pruning
cargo run -- levels.xsb 1 --deadlock-max-nodes 20  # Set corral search node limit
```

For better performance, use release mode:
```bash
cargo run --release -- levels.xsb 1
```

### Testing
```bash
cargo test                    # Run all tests
cargo test <test_name>        # Run a specific test
cargo test <module>::tests    # Run tests for a specific module
cargo test -- --nocapture     # Run tests with output visible
cargo test -- --test-threads=1 # Run tests sequentially
```

**Test Writing Convention**: Game board tests use multiline raw string literals with `r#"..."#` syntax for readability. A helper function `parse_game()` strips leading/trailing newlines. Example:
```rust
let game = parse_game(r#"
####
#@$.#
####
"#).unwrap();
```

### Linting and Formatting
```bash
cargo clippy         # Run Clippy linter
cargo fmt            # Format code
cargo fmt -- --check # Check formatting without modifying files
```

## Code Architecture

### Core Modules

- **game.rs**: Core Sokoban game state representation and move generation
  - `Game`: Represents the board state (tiles, player position, box positions, goals)
  - `Push` and `Pull`: Represent box push/pull moves (box index + direction)
  - `Moves<T>`: Bitset-based collection of valid moves (generic over Push/Pull)
  - Key methods: `compute_pushes()`, `compute_pulls()`, `push()`, `pull()`, `is_solved()`
  - Uses position canonicalization: normalizes player position to lexicographically smallest reachable position
  - Supports both forward pushes and backward pulls for bidirectional search
  - Computes dead squares (positions where a box can never reach any goal)
  - Implements PI-corral detection for advanced pruning

- **solver.rs**: IDA* search algorithm with bidirectional search support
  - `Solver`: Public API managing both forward and backward searchers
  - `Searcher`: Internal struct performing A* search up to a given threshold
  - `SearchHelper` trait: Abstracts forward vs backward search (implemented by `ForwardsSearchHelper` and `BackwardsSearchHelper`)
  - Supports three search types: Forward, Reverse, Bidirectional
  - Transposition table using Zobrist hashing to avoid revisiting states
  - Note: Bidirectional search is not guaranteed optimal (see solver.rs:505-510)
  - Returns `SolveResult` enum (Solved, Impossible, Cutoff)

- **heuristic.rs**: Heuristic functions for A* search
  - `Heuristic` trait: defines `new_push()`, `new_pull()`, and `compute()` methods
  - `SimpleHeuristic`: Simple assignment heuristic (admissible but slower)
  - `GreedyHeuristic`: Greedy assignment heuristic using counting sort (O(n²), not admissible but fast)
  - `HungarianHeuristic`: Optimal assignment heuristic using Hungarian algorithm (O(n³), admissible, default)
  - `NullHeuristic`: Returns 0 (reduces to iterative deepening)
  - All heuristics precompute push/pull distances from goals using BFS
  - Frozen boxes (boxes that cannot move) are excluded from heuristic computation

- **hungarian.rs**: Hungarian algorithm for minimum cost matching
  - Implements Kuhn-Munkres algorithm for optimal box-to-goal assignment
  - `Matrix` trait: abstraction for cost matrices
  - `ArrayMatrix`: Stack-allocated matrix using ArrayVec (no heap allocations)
  - Used by HungarianHeuristic to compute admissible lower bounds

- **zobrist.rs**: Zobrist hashing for game state identification
  - Pre-generates random hash values for each board position
  - Separate hash tables for box positions and player positions
  - Enables efficient incremental hash updates during search (XOR old position, XOR new position)

- **frozen.rs**: Freeze deadlock detection
  - `compute_frozen_boxes()`: Identifies boxes that cannot be moved due to surrounding walls/boxes
  - `compute_new_frozen_boxes()`: Incremental frozen box computation after a push
  - Used to detect when boxes form immovable structures
  - Freezing propagates: if a box is frozen and another box blocks it, that box also becomes frozen

- **corral.rs**: PI-corral deadlock detection
  - Implements "packing inside corral" deadlock detection
  - `Corral`: Represents a region of boxes that could potentially be trapped
  - `CorralSearcher`: Uses DFS to check if boxes in a corral can all reach goals
  - More expensive than freeze detection but catches additional deadlock patterns
  - Configurable node limit (default 20) for corral search depth

- **bits.rs**: Bit manipulation utilities
  - `Bitvector`: 64-bit bitvector for efficient set operations on box indices
  - `RawBitboard`: 64×64 bitboard for position-based checks (used for frozen boxes)
  - `LazyBitboard`: Lazily initialized bitboard for reachability calculations
  - `Index`: Type-safe wrapper for box indices
  - `Position`: Type-safe wrapper for (x, y) board positions

- **pqueue.rs**: Priority queue implementation
  - Custom binary heap optimized for the solver's needs
  - Used in A* search to track frontier nodes

- **levels.rs**: XSB format level file parsing
  - Parses levels where any line not starting with `#` (after optional spaces) is a separator
  - Empty lines, comment lines (`;`), and other text all separate levels
  - Returns collection of `Game` instances

### Important Design Details

1. **State Representation**: Game states are identified by box positions + canonicalized player position (not the actual player position). This significantly reduces the state space by treating all player positions in the same connected region as equivalent.

2. **Move Representation**: The solver works with box pushes/pulls rather than player moves. Each push/pull implicitly includes the player movement needed to reach the box.

3. **SearchHelper Trait Pattern**: The `Searcher` struct is generic over a `SearchHelper` trait, allowing the same search code to handle both forward search (using pushes) and backward search (using pulls). This enables bidirectional search without code duplication.

4. **Bidirectional Search**: When enabled, alternates between forward and backward search based on nodes explored. The searchers detect overlap by checking if a state exists in the other searcher's transposition table. **Important limitation**: Not guaranteed to find optimal solutions because A* doesn't explore in BFS order.

5. **Pruning Strategies**: All pruning techniques are independently configurable via CLI flags:
   - **Freeze deadlock detection** (enabled by default): Detects when boxes form immovable structures
   - **Dead square pruning** (enabled by default): Prunes moves to positions where boxes can never reach any goal
   - **PI-corral pruning** (enabled by default): Detects when boxes are trapped in regions they cannot escape

6. **Dead Square Detection**: On initialization, `compute_dead_squares()` performs backward reachability analysis from goal positions to identify squares where boxes can never reach any goal. Separate analysis for push-dead squares (forward search) and pull-dead squares (backward search).

7. **Zobrist Hashing**: Incremental hash updates are used during search. When a box moves, the hash is updated by XORing out the old position hash and XORing in the new position hash.

8. **Transposition Table**: Stores parent state hash and g-cost (depth) for each visited state. This enables solution reconstruction by following parent links backwards from the final state. States revisited at equal or greater depth are pruned.

9. **Constants**:
   - `MAX_SIZE = 64`: Maximum board dimension (x and y)
   - `MAX_BOXES = 64`: Maximum number of boxes (changed from 32, affects all stack-allocated arrays)
   - Default node limit: 5 million states
   - Default corral search node limit: 20 states

10. **Performance Optimizations**:
   - Stack allocation preferred over heap (ArrayVec used extensively)
   - Counting sort for heuristic computations (faster than built-in sorts for small ranges)
   - Lazy initialization of bitboards to avoid unnecessary allocations
   - Incremental hash updates to avoid full state rehashing
   - MaybeUninit for uninitialized arrays to avoid initialization overhead

### XSB Level File Format

Levels are separated by lines starting with `;` (comments) or empty lines. Standard Sokoban notation:
- `#` = Wall
- ` ` (space) = Floor
- `.` = Goal
- `$` = Box
- `@` = Player
- `*` = Box on goal
- `+` = Player on goal

## Rust Edition

This project uses Rust edition 2024, which requires Rust 1.85.0 or later.
