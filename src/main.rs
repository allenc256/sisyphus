mod game;
mod heuristic;
mod levels;
mod solver;
mod zobrist;

use clap::{Parser, ValueEnum};
use game::{Game, PushByPos};
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

fn print_solution(game: &Game, solution: &[PushByPos]) {
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

fn solve_level_with_heuristic<H: Heuristic>(
    level_num: usize,
    game: &Game,
    print_solution_flag: bool,
    max_nodes_explored: usize,
    heuristic: H,
    search_type: SearchType,
) {
    let mut solver = Solver::new(max_nodes_explored, heuristic, search_type, game);
    let start = Instant::now();
    let result = solver.solve(game);
    let elapsed = start.elapsed();

    let (solved_char, solution_len) = match &result {
        SolveResult::Solved(solution) => ('Y', solution.len()),
        SolveResult::Cutoff => ('N', 0),
        SolveResult::Impossible => ('I', 0),
    };

    println!(
        "level: {:<3}  solved: {}  steps: {:<3}  states: {:<10}  elapsed: {} ms",
        level_num,
        solved_char,
        solution_len,
        solver.nodes_explored(),
        elapsed.as_millis()
    );

    if print_solution_flag {
        if let SolveResult::Solved(solution) = result {
            print_solution(game, &solution);
        }
    }
}

fn solve_level(
    level_num: usize,
    game: &Game,
    print_solution_flag: bool,
    max_nodes_explored: usize,
    heuristic_type: HeuristicType,
    search_type: SearchType,
) {
    match heuristic_type {
        HeuristicType::Greedy => solve_level_with_heuristic(
            level_num,
            game,
            print_solution_flag,
            max_nodes_explored,
            GreedyHeuristic::new(),
            search_type,
        ),
        HeuristicType::Null => solve_level_with_heuristic(
            level_num,
            game,
            print_solution_flag,
            max_nodes_explored,
            NullHeuristic::new(),
            search_type,
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

    /// Search type (forwards, backwards, or bidirectional)
    #[arg(short = 'd', long, value_enum, default_value = "forwards")]
    direction: Direction,
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

    // Solve each level in the range
    for level_num in args.level_start..=level_end {
        let game = levels.get(level_num - 1).unwrap();
        solve_level(
            level_num,
            game,
            args.print_solution,
            args.max_nodes_explored,
            args.heuristic,
            args.direction.into(),
        );
    }
}
