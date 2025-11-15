mod bits;
mod deadlocks;
mod game;
mod heuristic;
mod levels;
mod solver;
mod zobrist;

use clap::{Parser, ValueEnum};
use game::{Game, MoveByPos};
use heuristic::{GreedyHeuristic, Heuristic, NullHeuristic};
use levels::Levels;
use solver::{SearchType, SolveResult, Solver};
use std::time::Instant;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum HeuristicType {
    Greedy,
    Null,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Direction {
    Forwards,
    Backwards,
    Bidirectional,
}

impl From<Direction> for SearchType {
    fn from(dir: Direction) -> Self {
        match dir {
            Direction::Forwards => SearchType::Forwards,
            Direction::Backwards => SearchType::Backwards,
            Direction::Bidirectional => SearchType::Bidirectional,
        }
    }
}

fn print_solution(game: &Game, solution: &[MoveByPos]) {
    println!("\nStarting position:\n{}", game);
    let mut game = game.clone();
    let mut count = 0;
    let total = solution.len();
    for push in solution {
        game.push_by_pos(*push);
        count += 1;
        println!(
            "Push crate ({}, {}) {} ({}/{}):\n{}",
            push.box_pos.0, push.box_pos.1, push.direction, count, total, game
        );
    }
}

struct LevelStats {
    solved: bool,
    steps: usize,
    states_explored: usize,
    elapsed_ms: u128,
}

fn solve_level_with_heuristic<H: Heuristic>(
    level_num: usize,
    game: &Game,
    print_solution_flag: bool,
    max_nodes_explored: usize,
    heuristic: H,
    search_type: SearchType,
    freeze_deadlocks: bool,
) -> LevelStats {
    let mut solver = Solver::new(
        max_nodes_explored,
        heuristic,
        search_type,
        game,
        freeze_deadlocks,
    );
    let start = Instant::now();
    let result = solver.solve();
    let elapsed = start.elapsed();
    let (nodes_forwards, nodes_backwards) = solver.nodes_explored();

    let total_states = nodes_forwards + nodes_backwards;
    let elapsed_ms = elapsed.as_millis();

    let (solved_char, solution_len, solved) = match &result {
        SolveResult::Solved(solution) => ('Y', solution.len(), true),
        SolveResult::Cutoff => ('N', 0, false),
        SolveResult::Impossible => ('X', 0, false),
    };

    println!(
        "level: {:<3}  solved: {}  steps: {:<5}  states: {:<12}  elapsed: {} ms",
        level_num, solved_char, solution_len, total_states, elapsed_ms
    );

    if print_solution_flag {
        if let SolveResult::Solved(solution) = result {
            print_solution(game, &solution);
        }
    }

    LevelStats {
        solved,
        steps: solution_len,
        states_explored: total_states,
        elapsed_ms,
    }
}

fn solve_level(
    level_num: usize,
    game: &Game,
    print_solution_flag: bool,
    max_nodes_explored: usize,
    heuristic_type: HeuristicType,
    search_type: SearchType,
    freeze_deadlocks: bool,
) -> LevelStats {
    match heuristic_type {
        HeuristicType::Greedy => solve_level_with_heuristic(
            level_num,
            game,
            print_solution_flag,
            max_nodes_explored,
            GreedyHeuristic::new(game),
            search_type,
            freeze_deadlocks,
        ),
        HeuristicType::Null => solve_level_with_heuristic(
            level_num,
            game,
            print_solution_flag,
            max_nodes_explored,
            NullHeuristic::new(),
            search_type,
            freeze_deadlocks,
        ),
    }
}

#[derive(Parser)]
#[command(name = "sisyphus")]
#[command(about = "A Sokoban solver", long_about = None)]
struct Args {
    /// Path to the levels file (XSB format)
    #[arg(value_name = "FILE")]
    levels_file: String,

    /// Level number to solve (1-indexed), or start of range
    #[arg(value_name = "LEVEL")]
    level_start: usize,

    /// Optional end of level range (inclusive, 1-indexed)
    #[arg(value_name = "LEVEL_END")]
    level_end: Option<usize>,

    /// Print the solution step-by-step
    #[arg(short, long)]
    print_solution: bool,

    /// Maximum number of nodes to explore before giving up
    #[arg(short = 'n', long, default_value = "5000000")]
    max_nodes_explored: usize,

    /// Heuristic to use for solving
    #[arg(short = 'H', long, value_enum, default_value = "greedy")]
    heuristic: HeuristicType,

    /// Search type
    #[arg(short = 'd', long, value_enum, default_value = "bidirectional")]
    direction: Direction,

    /// Disable freeze deadlock detection
    #[arg(long, default_value = "false")]
    no_freeze_deadlocks: bool,
}

fn main() {
    let args = Args::parse();

    // Load levels from file
    let levels = match Levels::from_file(&args.levels_file) {
        Ok(levels) => levels,
        Err(e) => {
            eprintln!("Error loading levels: {}", e);
            std::process::exit(1);
        }
    };

    // Determine the range of levels to solve
    let level_end = args.level_end.unwrap_or(args.level_start);
    let num_levels = level_end - args.level_start + 1;

    // Validate range
    if args.level_start == 0 {
        eprintln!("Error: level numbers must be at least 1");
        std::process::exit(1);
    }

    if level_end < args.level_start {
        eprintln!("Error: level end must be >= level start");
        std::process::exit(1);
    }

    if level_end > levels.len() {
        eprintln!(
            "Error: level {} not found (file contains {} levels)",
            level_end,
            levels.len()
        );
        std::process::exit(1);
    }

    if args.print_solution && num_levels > 1 {
        eprintln!("Error: solution printing only supported when solving a single level");
        std::process::exit(1);
    }

    // Solve each level in the range
    let mut total_solved = 0;
    let mut total_steps = 0;
    let mut total_states = 0;
    let mut total_time_ms = 0;

    for level_num in args.level_start..=level_end {
        let game = levels.get(level_num - 1).unwrap();
        let stats = solve_level(
            level_num,
            game,
            args.print_solution,
            args.max_nodes_explored,
            args.heuristic,
            args.direction.into(),
            !args.no_freeze_deadlocks,
        );

        if stats.solved {
            total_solved += 1;
        }
        total_steps += stats.steps;
        total_states += stats.states_explored;
        total_time_ms += stats.elapsed_ms;
    }

    // Print summary statistics if multiple levels were solved
    if num_levels > 1 {
        println!("---");
        println!(
            "solved: {:>3}/{:<3}        steps: {:<5}  states: {:<12}  elapsed: {} ms",
            total_solved, num_levels, total_steps, total_states, total_time_ms
        );
    }
}
