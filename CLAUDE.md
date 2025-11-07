# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Sisyphus is a Rust project (edition 2024) in early development. The codebase currently contains a minimal "Hello, world!" application.

## Development Commands

### Building
```bash
cargo build          # Build the project
cargo build --release # Build with optimizations
```

### Running
```bash
cargo run            # Build and run the project
```

### Testing
```bash
cargo test           # Run all tests
cargo test <test_name> # Run a specific test
cargo test -- --nocapture # Run tests with output visible
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

## Project Structure

Currently minimal:
- `src/main.rs` - Entry point with main function
- `Cargo.toml` - Package manifest (no dependencies yet)

## Rust Edition

This project uses Rust edition 2024, which requires Rust 1.85.0 or later.
