mod bits;
mod deadlocks;
mod game;
mod heuristic;
mod levels;
mod solver;
mod zobrist;

use clap::{Parser, ValueEnum};
use game::Game;
use heuristic::{GreedyHeuristic, Heuristic, NullHeuristic};
use levels::Levels;
use solver::{SearchType, SolveResult, Solver};
use std::time::Instant;

use crate::{
    game::{Move, Push},
    solver::Tracer,
};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum HeuristicType {
    Greedy,
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

struct VerboseTracer {
    trace_start: usize,
    trace_end: usize,
}

impl VerboseTracer {
    fn new(from_move: usize, to_move: usize) -> Self {
        Self {
            trace_start: from_move,
            trace_end: to_move,
        }
    }
}

impl Tracer for VerboseTracer {
    fn trace<M: Move>(
        &self,
        game: &Game,
        nodes_explored: usize,
        threshold: usize,
        f_cost: usize,
        g_cost: usize,
        move_: &M,
    ) {
        if self.trace_start <= nodes_explored && nodes_explored <= self.trace_end {
            println!(
                "move={}, pos={}, count={}, f_cost={}, g_cost={}, threshold={}:\n{}",
                move_,
                game.box_position(move_.box_index()),
                nodes_explored,
                f_cost,
                g_cost,
                threshold,
                game
            );
        }
    }
}

struct LevelStats {
    solved: bool,
    steps: usize,
    states_explored: usize,
    elapsed_ms: u128,
}

struct SolveOpts {
    level_num: usize,
    max_nodes_explored: usize,
    search_type: SearchType,
    print_solution: bool,
    freeze_deadlocks: bool,
    pi_corrals: bool,
    trace_range: Option<(usize, usize)>,
}

fn solve_level_helper<H: Heuristic>(
    game: &Game,
    opts: SolveOpts,
    forward_heuristic: H,
    reverse_heuristic: H,
) -> LevelStats {
    let tracer: Option<VerboseTracer> = if let Some((trace_start, trace_end)) = opts.trace_range {
        Some(VerboseTracer::new(trace_start, trace_end))
    } else {
        None
    };

    let mut solver = Solver::new(
        opts.max_nodes_explored,
        forward_heuristic,
        reverse_heuristic,
        opts.search_type,
        game.clone(),
        opts.freeze_deadlocks,
        opts.pi_corrals,
        tracer,
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
        opts.level_num, solved_char, solution_len, total_states, elapsed_ms
    );

    // if solved_char != 'Y' {
    //     for (hash, count) in solver.frozen_counts.iter() {
    //         println!("{:016x}: {}", hash, count);
    //     }
    // }

    if opts.print_solution {
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

fn solve_level(game: &Game, opts: SolveOpts, heuristic_type: HeuristicType) -> LevelStats {
    match heuristic_type {
        HeuristicType::Greedy => solve_level_helper(
            game,
            opts,
            GreedyHeuristic::new_forward(game),
            GreedyHeuristic::new_reverse(game),
        ),
        HeuristicType::Null => {
            solve_level_helper(game, opts, NullHeuristic::new(), NullHeuristic::new())
        }
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

    /// Disable PI-corral pruning
    #[arg(long, default_value = "false")]
    no_pi_corrals: bool,

    /// Range of move numbers to trace (start, end)
    #[arg(long, num_args = 2)]
    trace_range: Option<Vec<usize>>,
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

    // Validate trace_range
    if let Some(ref range) = args.trace_range {
        if range[0] > range[1] {
            eprintln!("Error: trace range start must be <= end");
            std::process::exit(1);
        }
    }

    // Solve each level in the range
    let mut total_solved = 0;
    let mut total_steps = 0;
    let mut total_states = 0;
    let mut total_time_ms = 0;

    // Parse trace_range from Vec to tuple
    let trace_range = args.trace_range.as_ref().map(|v| (v[0], v[1]));

    for level_num in args.level_start..=level_end {
        let game = levels.get(level_num - 1).unwrap();
        let opts = SolveOpts {
            level_num,
            max_nodes_explored: args.max_nodes_explored,
            search_type: args.direction.into(),
            print_solution: args.print_solution,
            freeze_deadlocks: !args.no_freeze_deadlocks,
            pi_corrals: !args.no_pi_corrals,
            trace_range,
        };
        let stats = solve_level(game, opts, args.heuristic);

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
