use crate::deadlocks::Deadlocks;
use crate::game::{Game, Move, MoveByPos, Moves, PlayerPos, Pull, Push};
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
struct Searcher<H: Heuristic, T: Tracer, S: SearchHelper> {
    nodes_explored: usize,
    max_nodes_explored: usize,
    table: HashMap<u64, TableEntry>, // Transposition table mapping state hash to entry
    zobrist: Rc<Zobrist>,
    heuristic: Rc<H>,
    initial_game: Rc<Game>,
    initial_hash: u64,
    initial_boxes_hash: u64,
    freeze_deadlocks: bool,
    tracer: Option<Rc<T>>,
    helper: S,
}

/// Internal trait containing search logic that is polymorphic depending on the
/// direction of the search (forward vs reverse).
trait SearchHelper {
    type Move: Move;

    fn compute_moves(&self, game: &Game) -> (Moves<Self::Move>, PlayerPos);
    fn compute_unmoves(&self, game: &Game) -> Moves<Self::Move>;
    fn apply_move(&self, game: &mut Game, move_: &Self::Move);
    fn unapply_move(&self, game: &mut Game, move_: &Self::Move);
    fn is_forwards_search(&self) -> bool;
}

struct ForwardsSearchHelper {
    pi_corrals: bool,
}

struct BackwardsSearchHelper;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchType {
    Forwards,
    Backwards,
    Bidirectional,
}

/// Manages iterative deepening A* by repeatedly calling Searcher with increasing thresholds
pub struct Solver<H: Heuristic, T: Tracer> {
    forwards: Searcher<H, T, ForwardsSearchHelper>,
    backwards: Searcher<H, T, BackwardsSearchHelper>,
    search_type: SearchType,
}

pub trait Tracer {
    fn trace_move<M: Move>(
        &self,
        is_forwards: bool,
        game: &Game,
        threshold: usize,
        f_cost: usize,
        g_cost: usize,
        move_: &M,
    );
}

impl<H: Heuristic, T: Tracer, S: SearchHelper> Searcher<H, T, S> {
    fn new(
        zobrist: Rc<Zobrist>,
        max_nodes_explored: usize,
        heuristic: Rc<H>,
        initial_game: Rc<Game>,
        freeze_deadlocks: bool,
        tracer: Option<Rc<T>>,
        helper: S,
    ) -> Self {
        let initial_hash = zobrist.compute_hash(&initial_game);
        let initial_boxes_hash = zobrist.compute_boxes_hash(&initial_game);
        let mut searcher = Searcher {
            nodes_explored: 0,
            max_nodes_explored,
            table: HashMap::new(),
            zobrist,
            heuristic,
            initial_game,
            initial_hash,
            initial_boxes_hash,
            freeze_deadlocks,
            tracer,
            helper,
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
        let (moves, canonical_pos) = self.helper.compute_moves(game);

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
            let old_box_pos = game.box_pos(move_.box_index() as usize);
            self.helper.apply_move(game, &move_);
            let new_box_pos = game.box_pos(move_.box_index() as usize);

            if let Some(tracer) = &self.tracer {
                tracer.trace_move(
                    self.helper.is_forwards_search(),
                    game,
                    threshold,
                    f_cost,
                    g_cost,
                    &move_,
                );
            }

            if self.freeze_deadlocks
                && Deadlocks::is_freeze_deadlock(game, new_box_pos.0, new_box_pos.1)
            {
                self.helper.unapply_move(game, &move_);
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

            self.helper.unapply_move(game, &move_);

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
            let unmoves = self.helper.compute_unmoves(&current_game);

            // Try each unmove to find which one leads to parent state
            let mut found = false;
            for unmove in &unmoves {
                let old_box_pos = current_game.box_pos(unmove.box_index() as usize);
                self.helper.unapply_move(&mut current_game, &unmove);
                let new_box_pos = current_game.box_pos(unmove.box_index() as usize);

                // Compute hash of this previous state
                let prev_hash = self.zobrist.compute_hash(&current_game);

                // Check if this matches the parent we're looking for
                if prev_hash == target_parent_hash {
                    solution.push(MoveByPos {
                        box_pos: if self.helper.is_forwards_search() {
                            new_box_pos
                        } else {
                            old_box_pos
                        },
                        direction: unmove.direction(),
                    });
                    current_hash = prev_hash;
                    found = true;
                    break;
                }

                // Redo the unmove if it wasn't correct
                self.helper.apply_move(&mut current_game, &unmove);
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
        if self.helper.is_forwards_search() {
            self.heuristic.compute_forward(game)
        } else {
            self.heuristic.compute_backward(game)
        }
    }
}

impl SearchHelper for ForwardsSearchHelper {
    type Move = Push;

    fn compute_moves(&self, game: &Game) -> (Moves<Push>, PlayerPos) {
        if self.pi_corrals {
            game.compute_pi_corral_pushes()
        } else {
            game.compute_pushes()
        }
    }

    fn compute_unmoves(&self, game: &Game) -> Moves<Push> {
        game.compute_pulls().0.to_pushes()
    }

    fn apply_move(&self, game: &mut Game, push: &Push) {
        game.push(*push);
    }

    fn unapply_move(&self, game: &mut Game, push: &Push) {
        game.pull(push.to_pull());
    }

    fn is_forwards_search(&self) -> bool {
        true
    }
}

impl SearchHelper for BackwardsSearchHelper {
    type Move = Pull;

    fn compute_moves(&self, game: &Game) -> (Moves<Pull>, PlayerPos) {
        game.compute_pulls()
    }

    fn compute_unmoves(&self, game: &Game) -> Moves<Pull> {
        game.compute_pushes().0.to_pulls()
    }

    fn apply_move(&self, game: &mut Game, pull: &Pull) {
        game.pull(*pull)
    }

    fn unapply_move(&self, game: &mut Game, pull: &Pull) {
        game.push(pull.to_push())
    }

    fn is_forwards_search(&self) -> bool {
        false
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
                forwards_game,
                freeze_deadlocks,
                tracer.clone(),
                ForwardsSearchHelper { pi_corrals },
            ),
            backwards: Searcher::new(
                zobrist,
                max_nodes_explored,
                heuristic,
                backwards_game,
                freeze_deadlocks,
                tracer,
                BackwardsSearchHelper,
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

            if is_forwards {
                match self.forwards.search(forwards_threshold, &|hash| {
                    self.backwards.table.contains_key(&hash)
                }) {
                    SearchResult::Solved(game) => {
                        return SolveResult::Solved(self.reconstruct_solution(&game));
                    }
                    SearchResult::Exceeded => forwards_threshold += 1,
                    SearchResult::Cutoff => return SolveResult::Cutoff,
                    SearchResult::Impossible => return SolveResult::Impossible,
                }
            } else {
                match self.backwards.search(backwards_threshold, &|hash| {
                    self.forwards.table.contains_key(&hash)
                }) {
                    SearchResult::Solved(game) => {
                        return SolveResult::Solved(self.reconstruct_solution(&game));
                    }
                    SearchResult::Exceeded => backwards_threshold += 1,
                    SearchResult::Cutoff => return SolveResult::Cutoff,
                    SearchResult::Impossible => return SolveResult::Impossible,
                }
            }
        }
    }

    fn reconstruct_solution(&self, game: &Game) -> Vec<MoveByPos> {
        let mut forwards_soln = self.forwards.reconstruct_solution(&game);
        let mut backwards_soln = self.backwards.reconstruct_solution(&game);
        backwards_soln.reverse();
        forwards_soln.extend_from_slice(&backwards_soln);
        self.verify_solution(&forwards_soln);
        forwards_soln
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
            let index_push = Push::new(box_index, push.direction);
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
        fn trace_move<M: Move>(
            &self,
            _is_forwards_search: bool,
            _game: &Game,
            _threshold: usize,
            _f_cost: usize,
            _g_cost: usize,
            _move: &M,
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
