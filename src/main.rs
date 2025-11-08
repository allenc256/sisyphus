mod game;
mod levels;
mod solver;

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

#[derive(Parser)]
#[command(name = "sisyphus")]
#[command(about = "A Sokoban solver", long_about = None)]
struct Args {
    /// Path to the levels file (XSB format)
    #[arg(value_name = "FILE")]
    levels_file: String,

    /// Level number to solve (1-indexed)
    #[arg(value_name = "LEVEL")]
    level_number: usize,

    /// Print the solution step-by-step
    #[arg(short, long)]
    print_solution: bool,
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

    // Get the specified level (converting from 1-indexed to 0-indexed)
    if args.level_number == 0 {
        eprintln!("Error: level_number must be at least 1");
        std::process::exit(1);
    }

    let game = match levels.get(args.level_number - 1) {
        Some(game) => game,
        None => {
            eprintln!(
                "Error: level {} not found (file contains {} levels)",
                args.level_number,
                levels.len()
            );
            std::process::exit(1);
        }
    };

    let mut solver = Solver::new();
    let start = Instant::now();
    let result = solver.solve(game);
    let elapsed = start.elapsed();

    if result.is_none() {
        println!(
            "No solution, {} states, {} ms",
            solver.nodes_explored(),
            elapsed.as_millis()
        );
        return;
    }

    let solution = result.unwrap();

    println!(
        "{} steps, {} states, {} ms",
        solution.len(),
        solver.nodes_explored(),
        elapsed.as_millis()
    );

    if args.print_solution {
        print_solution(game, &solution);
    }
}
