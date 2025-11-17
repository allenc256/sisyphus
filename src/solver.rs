use crate::deadlocks::Deadlocks;
use crate::game::{Game, Move, MoveByPos, Moves, PlayerPos};
use crate::heuristic::{Cost, Heuristic};
use crate::zobrist::Zobrist;
use std::collections::HashMap;
use std::rc::Rc;

/// Result of a search iteration
#[derive(Debug, Clone, PartialEq, Eq)]
enum SearchResult {
    /// Solution found
    Solved(Box<Game>),
    /// No solution at this threshold
    Exceeded,
    /// Node limit exceeded
    Cutoff,
    /// Puzzle is impossible to solve
    Impossible,
}

/// Result of solving a puzzle
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveResult {
    /// Puzzle was solved
    Solved(Vec<MoveByPos>),
    /// Node limit exceeded before solution found
    Cutoff,
    /// Puzzle is impossible to solve
    Impossible,
}

/// Entry in the transposition table
#[derive(Debug, Clone, Copy)]
struct TableEntry {
    parent_hash: u64,
    g_cost: usize,
}

/// Performs A* search up to a specified threshold
struct Searcher<H: Heuristic, T: Tracer> {
    nodes_explored: usize,
    max_nodes_explored: usize,
    table: HashMap<u64, TableEntry>, // Transposition table mapping state hash to entry
    zobrist: Rc<Zobrist>,
    heuristic: Rc<H>,
    direction: SearchDirection,
    initial_game: Rc<Game>,
    initial_hash: u64,
    initial_boxes_hash: u64,
    freeze_deadlocks: bool,
    pi_corrals: bool,
    tracer: Option<Rc<T>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    Forwards,
    Backwards,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchType {
    Forwards,
    Backwards,
    Bidirectional,
}

/// Manages iterative deepening A* by repeatedly calling Searcher with increasing thresholds
pub struct Solver<H: Heuristic, T: Tracer> {
    forwards: Searcher<H, T>,
    backwards: Searcher<H, T>,
    search_type: SearchType,
}

pub trait Tracer {
    fn trace_move(
        &self,
        search_dir: SearchDirection,
        game: &Game,
        threshold: usize,
        f_cost: usize,
        g_cost: usize,
        move_: Move,
    );
}

impl<H: Heuristic, T: Tracer> Searcher<H, T> {
    fn new(
        zobrist: Rc<Zobrist>,
        max_nodes_explored: usize,
        heuristic: Rc<H>,
        direction: SearchDirection,
        initial_game: Rc<Game>,
        freeze_deadlocks: bool,
        pi_corrals: bool,
        tracer: Option<Rc<T>>,
    ) -> Self {
        let initial_hash = zobrist.compute_hash(&initial_game);
        let initial_boxes_hash = zobrist.compute_boxes_hash(&initial_game);
        // Only check freeze deadlocks and PI-corrals for forward search
        let freeze_deadlocks = freeze_deadlocks && direction == SearchDirection::Forwards;
        let pi_corrals = pi_corrals && direction == SearchDirection::Forwards;
        let mut searcher = Searcher {
            nodes_explored: 0,
            max_nodes_explored,
            table: HashMap::new(),
            zobrist,
            heuristic,
            direction,
            initial_game,
            initial_hash,
            initial_boxes_hash,
            freeze_deadlocks,
            pi_corrals,
            tracer,
        };
        searcher.reset();
        searcher
    }

    fn nodes_explored(&self) -> usize {
        self.nodes_explored
    }

    /// Perform A* search up to the specified threshold starting from the initial game state
    /// Returns SearchResult. If found, use reconstruct_solution to get the solution path.
    fn search<F>(&mut self, threshold: usize, target_check: &F) -> SearchResult
    where
        F: Fn(u64) -> bool,
    {
        let mut game = (*self.initial_game).clone();
        self.reset();
        self.search_helper(
            &mut game,
            0,
            threshold,
            target_check,
            self.initial_boxes_hash,
            0,
        )
    }

    fn reset(&mut self) {
        self.table.clear();

        // Important: bidirectional search relies on the invariant that the
        // transposition table always contains the initial state (since when
        // we're doing a forward search we're checking the backward search's
        // table to see if we've finished, and vice versa).
        self.table.insert(
            self.initial_hash,
            TableEntry {
                parent_hash: 0,
                g_cost: 0,
            },
        );
    }

    /// Perform DFS A* search up to the specified threshold
    /// Returns SearchResult. If found, use reconstruct_solution to get the solution path.
    fn search_helper<F>(
        &mut self,
        game: &mut Game,
        g_cost: usize,
        threshold: usize,
        target_check: &F,
        boxes_hash: u64,
        parent_hash: u64,
    ) -> SearchResult
    where
        F: Fn(u64) -> bool,
    {
        self.nodes_explored += 1;

        // Check if we've exceeded the node limit
        if self.nodes_explored > self.max_nodes_explored {
            return SearchResult::Cutoff;
        }

        // Compute heuristic and f-cost
        let mut f_cost = g_cost;
        match self.compute_heuristic(game) {
            Cost::Solvable(h_cost) => f_cost += h_cost as usize,
            Cost::Impossible => return SearchResult::Impossible,
        }

        // If f-cost exceeds threshold, stop searching this branch
        if f_cost > threshold {
            return SearchResult::Exceeded;
        }

        // Get all valid pushes and canonical position
        let (moves, canonical_pos) = self.compute_moves(game);

        // Hash in the canonical player position
        let curr_hash = boxes_hash ^ self.zobrist.player_hash(canonical_pos);

        // Check transposition table
        if let Some(entry) = self.table.get(&curr_hash) {
            // Skip if we've seen this state at a shallower or equal g-cost
            if g_cost > 0 && g_cost >= entry.g_cost {
                return SearchResult::Exceeded;
            }
        }

        // Mark this state as visited
        self.table.insert(
            curr_hash,
            TableEntry {
                parent_hash,
                g_cost,
            },
        );

        // Check if we've reached the target (after adding to transposition table)
        if target_check(curr_hash) {
            return SearchResult::Solved(Box::new(game.clone()));
        }

        let mut result = SearchResult::Impossible;

        // Try each push
        for move_ in &moves {
            let old_box_pos = game.box_pos(move_.box_index as usize);
            self.apply_move(game, move_);
            let new_box_pos = game.box_pos(move_.box_index as usize);

            if let Some(tracer) = &self.tracer {
                tracer.trace_move(self.direction, game, threshold, f_cost, g_cost, move_);
            }

            if self.freeze_deadlocks
                && Deadlocks::is_freeze_deadlock(game, new_box_pos.0, new_box_pos.1)
            {
                self.unapply_move(game, move_);
                continue;
            }

            // Update boxes hash (unhash old position, hash new position)
            let new_boxes_hash = boxes_hash
                ^ self.zobrist.box_hash(old_box_pos.0, old_box_pos.1)
                ^ self.zobrist.box_hash(new_box_pos.0, new_box_pos.1);

            let child_result = self.search_helper(
                game,
                g_cost + 1,
                threshold,
                target_check,
                new_boxes_hash,
                curr_hash,
            );
            if let SearchResult::Solved(_) = child_result {
                return child_result;
            }

            self.unapply_move(game, move_);

            if child_result == SearchResult::Cutoff {
                return child_result;
            }
            if child_result == SearchResult::Exceeded {
                result = SearchResult::Exceeded;
            }
        }

        result
    }

    fn reconstruct_solution(&self, final_game: &Game) -> Vec<MoveByPos> {
        let mut solution = Vec::new();
        let mut current_game = final_game.clone();
        let mut current_hash = self.zobrist.compute_hash(&current_game);

        // Work backwards until we reach the initial state (g_cost == 0)
        loop {
            let entry = self
                .table
                .get(&current_hash)
                .expect("Failed to reconstruct solution: state not in transposition table");

            if entry.g_cost == 0 {
                // Reached initial state
                break;
            }

            let target_parent_hash = entry.parent_hash;

            // Compute all possible unmoves from current state
            let (unmoves, _canonical_pos) = self.compute_unmoves(&current_game);

            // Try each unmove to find which one leads to parent state
            let mut found = false;
            for unmove in &unmoves {
                let old_box_pos = current_game.box_pos(unmove.box_index as usize);
                self.unapply_move(&mut current_game, unmove);
                let new_box_pos = current_game.box_pos(unmove.box_index as usize);

                // Compute hash of this previous state
                let prev_hash = self.zobrist.compute_hash(&current_game);

                // Check if this matches the parent we're looking for
                if prev_hash == target_parent_hash {
                    solution.push(MoveByPos {
                        box_pos: match self.direction {
                            SearchDirection::Forwards => new_box_pos,
                            SearchDirection::Backwards => old_box_pos,
                        },
                        direction: unmove.direction,
                    });
                    current_hash = prev_hash;
                    found = true;
                    break;
                }

                // Redo the unmove if it wasn't correct
                self.apply_move(&mut current_game, unmove);
            }

            assert!(
                found,
                "Failed to reconstruct solution: no unmove leads to parent state"
            );
        }

        solution.reverse();
        solution
    }

    fn compute_heuristic(&self, game: &Game) -> Cost {
        match self.direction {
            SearchDirection::Forwards => self.heuristic.compute_forward(game),
            SearchDirection::Backwards => self.heuristic.compute_backward(game),
        }
    }

    fn compute_moves(&self, game: &Game) -> (Moves, PlayerPos) {
        match self.direction {
            SearchDirection::Forwards => {
                if self.pi_corrals {
                    game.compute_pi_corral_pushes()
                } else {
                    game.compute_pushes()
                }
            }
            SearchDirection::Backwards => game.compute_pulls(),
        }
    }

    fn compute_unmoves(&self, game: &Game) -> (Moves, PlayerPos) {
        match self.direction {
            SearchDirection::Forwards => game.compute_pulls(),
            SearchDirection::Backwards => game.compute_pushes(),
        }
    }

    fn apply_move(&self, game: &mut Game, move_: Move) {
        match self.direction {
            SearchDirection::Forwards => game.push(move_),
            SearchDirection::Backwards => game.pull(move_),
        }
    }

    fn unapply_move(&self, game: &mut Game, move_: Move) {
        match self.direction {
            SearchDirection::Forwards => game.pull(move_),
            SearchDirection::Backwards => game.push(move_),
        }
    }
}

impl<H: Heuristic, T: Tracer> Solver<H, T> {
    pub fn new(
        max_nodes_explored: usize,
        heuristic: H,
        search_type: SearchType,
        game: &Game,
        freeze_deadlocks: bool,
        pi_corrals: bool,
        tracer: Option<T>,
    ) -> Self {
        let zobrist = Rc::new(Zobrist::new());
        let heuristic = Rc::new(heuristic);
        let tracer = tracer.map(|t| Rc::new(t));
        let forwards_game = Rc::new(game.clone());
        let backwards_game = Rc::new(game.make_goal_state());

        Solver {
            forwards: Searcher::new(
                zobrist.clone(),
                max_nodes_explored,
                heuristic.clone(),
                SearchDirection::Forwards,
                forwards_game,
                freeze_deadlocks,
                pi_corrals,
                tracer.clone(),
            ),
            backwards: Searcher::new(
                zobrist,
                max_nodes_explored,
                heuristic,
                SearchDirection::Backwards,
                backwards_game,
                freeze_deadlocks,
                pi_corrals,
                tracer,
            ),
            search_type,
        }
    }

    // Implements bidirectional search. Note that this implementation does not
    // guarantee optimal solution paths in general. This is because the A*
    // search going from either end is not guaranteed to explore the states in
    // order of BFS distance. This means that when we find a solution (i.e.,
    // when the forwards and backwards searchers overlap), the combined solution
    // might not be optimal.
    pub fn solve(&mut self) -> SolveResult {
        let mut forwards_threshold = 0;
        let mut backwards_threshold = 0;

        loop {
            let is_forwards = match self.search_type {
                SearchType::Forwards => true,
                SearchType::Backwards => false,
                SearchType::Bidirectional => {
                    self.forwards.nodes_explored() <= self.backwards.nodes_explored()
                }
            };

            let (active, inactive) = if is_forwards {
                (&mut self.forwards, &mut self.backwards)
            } else {
                (&mut self.backwards, &mut self.forwards)
            };

            let threshold = if is_forwards {
                &mut forwards_threshold
            } else {
                &mut backwards_threshold
            };

            match active.search(*threshold, &|hash| inactive.table.contains_key(&hash)) {
                SearchResult::Solved(final_game) => {
                    let (mut forwards_soln, mut backwards_soln) = if is_forwards {
                        (
                            active.reconstruct_solution(&final_game),
                            inactive.reconstruct_solution(&final_game),
                        )
                    } else {
                        (
                            inactive.reconstruct_solution(&final_game),
                            active.reconstruct_solution(&final_game),
                        )
                    };
                    backwards_soln.reverse();
                    forwards_soln.extend_from_slice(&backwards_soln);
                    self.verify_solution(&forwards_soln);
                    return SolveResult::Solved(forwards_soln);
                }
                SearchResult::Exceeded => *threshold += 1,
                SearchResult::Cutoff => return SolveResult::Cutoff,
                SearchResult::Impossible => return SolveResult::Impossible,
            }
        }
    }

    pub fn nodes_explored(&self) -> (usize, usize) {
        (
            self.forwards.nodes_explored(),
            self.backwards.nodes_explored(),
        )
    }

    fn verify_solution(&self, solution: &[MoveByPos]) {
        let mut test_game = (*self.forwards.initial_game).clone();

        for (i, push) in solution.iter().enumerate() {
            // Get box index at this position
            let box_index = test_game
                .box_at(push.box_pos.0, push.box_pos.1)
                .unwrap_or_else(|| {
                    panic!(
                        "Solution verification failed: no box at position ({}, {}) for push {}",
                        push.box_pos.0,
                        push.box_pos.1,
                        i + 1
                    )
                });

            // Compute valid pushes at this state
            let (valid_pushes, _canonical_pos) = test_game.compute_pushes();

            // Verify that this push is among the valid pushes
            let index_push = Move {
                box_index,
                direction: push.direction,
            };
            assert!(
                valid_pushes.contains(index_push),
                "Solution verification failed: push {} (box at ({}, {}), direction {:?}) is not valid",
                i + 1,
                push.box_pos.0,
                push.box_pos.1,
                push.direction
            );

            // Apply the push
            test_game.push_by_pos(*push);
        }

        // Verify final state is solved
        assert!(
            test_game.is_solved(),
            "Solution verification failed: after {} pushes, puzzle is not solved",
            solution.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::heuristic::GreedyHeuristic;

    use super::*;

    #[test]
    fn test_solve_simple() {
        let input = "####\n\
                     #@$.#\n\
                     ####";
        let game = Game::from_text(input).unwrap();
        let mut solver = new_solver(&game);
        let result = solver.solve();

        assert!(matches!(result, SolveResult::Solved(_)));
        if let SolveResult::Solved(moves) = result {
            assert_eq!(moves.len(), 1);

            // Verify solution works
            let mut test_game = Game::from_text(input).unwrap();
            for push in moves {
                test_game.push_by_pos(push);
            }
            assert!(test_game.is_solved());
        }
    }

    #[test]
    fn test_solve_already_solved() {
        let input = "####\n\
                     #@*#\n\
                     ####";
        let game = Game::from_text(input).unwrap();
        let mut solver = new_solver(&game);
        let result = solver.solve();

        assert!(matches!(result, SolveResult::Solved(_)));
        if let SolveResult::Solved(moves) = result {
            assert_eq!(moves.len(), 0);
        }
    }

    #[test]
    fn test_solve_two_moves() {
        let input = "#####\n\
                     #@$ .#\n\
                     #####";
        let game = Game::from_text(input).unwrap();
        let mut solver = new_solver(&game);
        let result = solver.solve();

        assert!(matches!(result, SolveResult::Solved(_)));
        if let SolveResult::Solved(moves) = result {
            assert_eq!(moves.len(), 2);

            // Verify solution works
            let mut test_game = Game::from_text(input).unwrap();
            for push in moves {
                test_game.push_by_pos(push);
            }
            assert!(test_game.is_solved());
        }
    }

    #[test]
    fn test_solve_impossible() {
        let input = "#####\n\
                     #@$#.#\n\
                     #####";
        let game = Game::from_text(input).unwrap();
        let mut solver = new_solver(&game);
        let result = solver.solve();
        assert_eq!(result, SolveResult::Impossible);
    }

    struct NullTracer {}

    impl Tracer for NullTracer {
        fn trace_move(
            &self,
            _search_dir: SearchDirection,
            _game: &Game,
            _threshold: usize,
            _f_cost: usize,
            _g_cost: usize,
            _move: Move,
        ) {
            unreachable!()
        }
    }

    fn new_solver(game: &Game) -> Solver<GreedyHeuristic, NullTracer> {
        Solver::new(
            5000000,
            GreedyHeuristic::new(game),
            SearchType::Forwards,
            game,
            true,
            true,
            None,
        )
    }
}
