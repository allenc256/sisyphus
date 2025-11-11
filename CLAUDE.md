# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Sisyphus is a Sokoban puzzle solver written in Rust (edition 2024). It uses the IDA* (Iterative Deepening A*) search algorithm with Zobrist hashing-based transposition tables to find optimal solutions to Sokoban puzzles.

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
cargo run -- levels.xsb 1 -d forwards  # Search forwards from initial state (default)
cargo run -- levels.xsb 1 -d backwards # Search backwards from goal state
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

### Other Useful Commands
```bash
cargo check          # Quick compile check without building
cargo clean          # Remove target directory
cargo doc --open     # Build and open documentation
```

## Code Architecture

### Core Modules

- **game.rs**: Core Sokoban game state representation and logic
  - `Game`: Represents the board state (tiles, player position, box positions, goals)
  - `PlayerPos`: Enum representing player position as either Known(x, y) or Unknown
  - `Push`: Represents a box push move (box index + direction)
  - `Pushes`: Bitset-based collection of valid pushes
  - Key methods: `compute_pushes()`, `compute_unpushes()`, `push()`, `unpush()`, `is_solved()`, `set_to_goal_state()`
  - Uses position canonicalization: normalizes player position to lexicographically smallest reachable position
  - Supports both forward pushes and backward unpushes for bidirectional search

- **solver.rs**: IDA* search algorithm implementation
  - Uses iterative deepening with increasing f-cost thresholds
  - Transposition table using Zobrist hashing to avoid revisiting states
  - Node limit to prevent excessive search (default: 5 million)
  - Returns `Option<Vec<Push>>` with solution path or None
  - Supports both forward search (from initial state) and backward search (from goal state)
  - `SearchDirection`: Controls whether to search forwards or backwards
  - `Searcher`: Internal struct that performs A* search up to a threshold
  - `Solver`: Public API that manages iterative deepening by repeatedly calling Searcher

- **heuristic.rs**: Heuristic functions for A* search
  - `Heuristic` trait: defines `compute_forward()` and `compute_backward()` methods
  - `GreedyHeuristic`: Greedy assignment heuristic using Manhattan distance (not admissible)
  - `NullHeuristic`: Returns 0 (reduces to iterative deepening)
  - Both methods support forward search (estimating distance to goal) and backward search (estimating distance from start)

- **zobrist.rs**: Zobrist hashing for game state identification
  - Pre-generates random hash values for each board position
  - Separate hash tables for box positions and player positions (including Unknown positions)
  - Enables efficient incremental hash updates during search
  - Supports hashing of PlayerPos enum (Known positions and Unknown position)

- **levels.rs**: XSB format level file parsing
  - Parses levels separated by semicolon-prefixed comments or empty lines
  - Returns collection of `Game` instances

### Important Design Details

1. **State Representation**: Game states are identified by box positions + canonicalized player position (not the actual player position). This significantly reduces the state space. The player position can be either Known(x, y) or Unknown (used for goal states in backward search).

2. **Move Representation**: The solver works with box pushes rather than player moves. Each push implicitly includes the player movement needed to reach the box. The solver supports both forward pushes (moving boxes toward goals) and backward unpushes (moving boxes away from goals for backward search).

3. **Zobrist Hashing**: Incremental hash updates are used during search. When a box moves, the hash is updated by XORing out the old position hash and XORing in the new position hash.

4. **Transposition Table**: Stores both the parent state hash and g-cost (depth) for each visited state. This enables solution reconstruction by following parent links backwards from the final state. States revisited at equal or greater depth are pruned.

5. **Constants**:
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
