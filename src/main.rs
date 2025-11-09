mod game;
mod levels;
mod solver;
mod zobrist;

use clap::Parser;
use game::Game;
use levels::Levels;
use solver::Solver;
use std::time::Instant;

fn print_solution(game: &Game, solution: &[game::Push]) {
    println!();
    println!("{}", game);
    let mut game = game.clone();
    let mut count = 0;
    let total = solution.len();
    for push in solution {
        game.push(*push);
        count += 1;

        println!();
        println!(
            "Push crate #{} {} ({}/{})",
            push.box_index + 1,
            push.direction,
            count,
            total
        );
        println!();
        println!("{}", game);
    }
}

fn solve_level(
    level_num: usize,
    game: &Game,
    print_solution_flag: bool,
    max_nodes_explored: usize,
) {
    let mut solver = Solver::new(max_nodes_explored);
    let start = Instant::now();
    let result = solver.solve(game);
    let elapsed = start.elapsed();
    let solution_len = result.as_ref().map(|v| v.len()).unwrap_or(0);
    let solved = if solution_len > 0 { "Y" } else { "N" };

    println!(
        "level: {:<3}  solved: {}  steps: {:<3}  states: {:<10}  elapsed: {} ms",
        level_num + 1,
        solved,
        solution_len,
        solver.nodes_explored(),
        elapsed.as_millis()
    );

    if solution_len > 0 && print_solution_flag {
        let solution = result.unwrap();
        print_solution(game, &solution);
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
        );
    }
}
