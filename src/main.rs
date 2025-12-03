mod bits;
mod corral;
mod frozen;
mod game;
mod heuristic;
mod hungarian;
mod levels;
mod pqueue;
mod solver;
mod zobrist;

use clap::{Parser, ValueEnum};
use game::Game;
use heuristic::{Heuristic, NullHeuristic, SimpleHeuristic};
use levels::Levels;
use solver::{SearchType, SolveResult, Solver};
use std::ops::Range;
use std::time::Instant;

use crate::{
    game::{Move, Push},
    heuristic::{GreedyHeuristic, HungarianHeuristic},
    solver::SolverOpts,
};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum HeuristicType {
    Simple,
    Greedy,
    Hungarian,
    Null,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Direction {
    Forward,
    Reverse,
    Bidirectional,
}

impl From<Direction> for SearchType {
    fn from(dir: Direction) -> Self {
        match dir {
            Direction::Forward => SearchType::Forward,
            Direction::Reverse => SearchType::Reverse,
            Direction::Bidirectional => SearchType::Bidirectional,
        }
    }
}

fn print_solution(game: &Game, solution: &[Push]) {
    println!("\nStarting position:\n{}", game);
    let mut game = game.clone();
    let mut count = 0;
    let total = solution.len();
    for push in solution {
        let box_pos = game.box_position(push.box_index());
        game.push(*push);
        count += 1;
        println!(
            "Push crate #{} {} {} ({}/{}):\n{}",
            push.box_index().0 + 1,
            box_pos,
            push.direction(),
            count,
            total,
            game
        );
    }
}

struct LevelStats {
    solved: bool,
    steps: usize,
    states_explored: usize,
    elapsed_ms: u128,
}

fn solve_level_helper<H: Heuristic>(
    game: &Game,
    level_num: usize,
    opts: SolverOpts,
    print_solution: bool,
) -> LevelStats {
    let mut solver = Solver::<H>::new(game, opts);
    let start = Instant::now();
    let (result, nodes_explored) = solver.solve();
    let elapsed = start.elapsed();

    let elapsed_ms = elapsed.as_millis();

    let (solved_char, solution_len, solved) = match &result {
        SolveResult::Solved(solution) => ('Y', solution.len(), true),
        SolveResult::Cutoff => ('N', 0, false),
        SolveResult::Unsolvable => ('X', 0, false),
    };

    println!(
        "level: {:<3}  solved: {}  steps: {:<5}  states: {:<12}  elapsed: {} ms",
        level_num, solved_char, solution_len, nodes_explored, elapsed_ms
    );

    // if solved_char != 'Y' {
    //     for (hash, count) in solver.frozen_counts.iter() {
    //         println!("{:016x}: {}", hash, count);
    //     }
    // }

    if print_solution {
        if let SolveResult::Solved(solution) = result {
            crate::print_solution(game, &solution);
        }
    }

    LevelStats {
        solved,
        steps: solution_len,
        states_explored: nodes_explored,
        elapsed_ms,
    }
}

fn solve_level(
    game: &Game,
    level_num: usize,
    opts: SolverOpts,
    heuristic_type: HeuristicType,
    print_solution: bool,
) -> LevelStats {
    match heuristic_type {
        HeuristicType::Simple => {
            solve_level_helper::<SimpleHeuristic>(game, level_num, opts, print_solution)
        }
        HeuristicType::Greedy => {
            solve_level_helper::<GreedyHeuristic>(game, level_num, opts, print_solution)
        }
        HeuristicType::Hungarian => {
            solve_level_helper::<HungarianHeuristic>(game, level_num, opts, print_solution)
        }
        HeuristicType::Null => {
            solve_level_helper::<NullHeuristic>(game, level_num, opts, print_solution)
        }
    }
}

fn parse_trace_range(s: &str) -> Result<Range<usize>, String> {
    // Try parsing as "start..=end" (inclusive)
    if let Some((start, end)) = s.split_once("..=") {
        let start: usize = start
            .parse()
            .map_err(|_| format!("invalid start: {}", start))?;
        let end: usize = end.parse().map_err(|_| format!("invalid end: {}", end))?;
        if start > end {
            return Err("start must be <= end".to_string());
        }
        return Ok(start..end + 1);
    }

    // Try parsing as "start..end" (exclusive)
    if let Some((start, end)) = s.split_once("..") {
        let start: usize = start
            .parse()
            .map_err(|_| format!("invalid start: {}", start))?;
        let end: usize = end.parse().map_err(|_| format!("invalid end: {}", end))?;
        if start > end {
            return Err("start must be <= end".to_string());
        }
        return Ok(start..end);
    }

    // Try parsing as a single integer
    let n: usize = s.parse().map_err(|_| format!("invalid value: {}", s))?;
    Ok(n..n + 1)
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
    max_nodes: usize,

    /// Heuristic to use for solving
    #[arg(short = 'H', long, value_enum, default_value = "hungarian")]
    heuristic: HeuristicType,

    /// Search type
    #[arg(short = 'd', long, value_enum, default_value = "bidirectional")]
    direction: Direction,

    /// Disable freeze deadlock detection
    #[arg(long, default_value = "false")]
    no_freeze_deadlocks: bool,

    /// Disable dead square pruning
    #[arg(long, default_value = "false")]
    no_dead_squares: bool,

    /// Disable PI-corral pruning
    #[arg(long, default_value = "false")]
    no_pi_corrals: bool,

    /// Maximum nodes to explore when searching for corral deadlocks
    #[arg(long, default_value = "0")]
    deadlock_max_nodes: usize,

    /// Range of node counts to trace (e.g., "100..200", "100..=200", or "100")
    #[arg(long, value_parser = parse_trace_range)]
    trace_range: Option<Range<usize>>,
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

    // Use 0..0 for no tracing
    let trace_range = args.trace_range.unwrap_or(0..0);

    for level_num in args.level_start..=level_end {
        let game = levels.get(level_num - 1).unwrap();
        let opts = SolverOpts {
            search_type: args.direction.into(),
            max_nodes_explored: args.max_nodes,
            freeze_deadlocks: !args.no_freeze_deadlocks,
            dead_squares: !args.no_dead_squares,
            pi_corrals: !args.no_pi_corrals,
            deadlock_max_nodes: args.deadlock_max_nodes,
            trace_range: trace_range.clone(),
        };
        let stats = solve_level(game, level_num, opts, args.heuristic, args.print_solution);

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
