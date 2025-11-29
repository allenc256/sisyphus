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
cargo run -- levels.xsb 1 -H greedy    # Use greedy heuristic (default)
cargo run -- levels.xsb 1 -d forward   # Search forwards from initial state (default)
cargo run -- levels.xsb 1 -d reverse   # Search backwards from goal state
cargo run -- levels.xsb 1 -d bidirectional  # Bidirectional search
cargo run -- levels.xsb 1 -p none      # No pruning
cargo run -- levels.xsb 1 -p dead-squares   # Dead square pruning (default)
cargo run -- levels.xsb 1 -p pi-corrals     # PI-corral pruning (includes dead squares)
```

For better performance, use release mode:
```bash
cargo run --release -- levels.xsb 1
```

### Testing
```bash
cargo test                    # Run all tests
cargo test <test_name>        # Run a specific test
cargo test -- --nocapture     # Run tests with output visible
cargo test -- --test-threads=1 # Run tests sequentially
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
  - `Heuristic` trait: defines `new_push()`, `new_pull()`, `compute_forward()`, and `compute_backward()` methods
  - `GreedyHeuristic`: Greedy assignment heuristic using Manhattan distance (not admissible, but fast)
  - `SimpleHeuristic`: Simple assignment heuristic (admissible, slower)
  - `NullHeuristic`: Returns 0 (reduces to iterative deepening)
  - Supports heuristic caching for performance

- **zobrist.rs**: Zobrist hashing for game state identification
  - Pre-generates random hash values for each board position
  - Separate hash tables for box positions and player positions
  - Enables efficient incremental hash updates during search

- **deadlocks.rs**: Deadlock detection for pruning unsolvable states
  - `compute_frozen_boxes()`: Identifies boxes that cannot be moved
  - `compute_new_frozen_boxes()`: Incremental frozen box computation after a push
  - Used to prune states where boxes are in positions they cannot escape from

- **bits.rs**: Bit manipulation utilities
  - `Bitvector`: 64-bit bitvector for efficient set operations on box indices
  - `Index`: Type-safe wrapper for box indices
  - `Position`: Type-safe wrapper for (x, y) board positions

- **levels.rs**: XSB format level file parsing
  - Parses levels separated by semicolon-prefixed comments or empty lines
  - Returns collection of `Game` instances

### Important Design Details

1. **State Representation**: Game states are identified by box positions + canonicalized player position (not the actual player position). This significantly reduces the state space by treating all player positions in the same connected region as equivalent.

2. **Move Representation**: The solver works with box pushes/pulls rather than player moves. Each push/pull implicitly includes the player movement needed to reach the box.

3. **SearchHelper Trait Pattern**: The `Searcher` struct is generic over a `SearchHelper` trait, allowing the same search code to handle both forward search (using pushes) and backward search (using pulls). This enables bidirectional search without code duplication.

4. **Bidirectional Search**: When enabled, alternates between forward and backward search based on nodes explored. The searchers detect overlap by checking if a state exists in the other searcher's transposition table. **Important limitation**: Not guaranteed to find optimal solutions because A* doesn't explore in BFS order.

5. **Pruning Strategies**:
   - `Pruning::None`: No pruning
   - `Pruning::DeadSquares`: Prunes moves to positions where boxes can never reach any goal
   - `Pruning::PiCorrals`: Advanced pruning using PI-corral detection (implies dead square pruning)

6. **Dead Square Detection**: On initialization, `compute_dead_squares()` performs backward reachability analysis from goal positions to identify squares where boxes can never reach any goal. Separate analysis for push-dead squares (forward search) and pull-dead squares (backward search).

7. **Zobrist Hashing**: Incremental hash updates are used during search. When a box moves, the hash is updated by XORing out the old position hash and XORing in the new position hash.

8. **Transposition Table**: Stores parent state hash and g-cost (depth) for each visited state. This enables solution reconstruction by following parent links backwards from the final state. States revisited at equal or greater depth are pruned.

9. **Constants**:
   - `MAX_SIZE = 64`: Maximum board dimension
   - `MAX_BOXES = 32`: Maximum number of boxes
   - Default node limit: 5 million states

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
