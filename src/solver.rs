use crate::bits::{Bitvector, Index};
use crate::deadlocks::{compute_frozen_boxes, compute_new_frozen_boxes};
use crate::game::{Direction, Game, Move, Moves, Position, Pull, Push};
use crate::heuristic::Heuristic;
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
    Solved(Vec<Push>),
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
    heuristic: H,
    initial_game: Game,
    initial_player_positions: Vec<Position>,
    initial_boxes_hash: u64,
    freeze_deadlocks: bool,
    // frozen_counts: HashMap<u64, usize>,
    tracer: Option<Rc<T>>,
    helper: S,
}

/// Internal trait containing search logic that is polymorphic depending on the
/// direction of the search (forward vs reverse).
trait SearchHelper {
    type Move: Move;

    fn compute_moves(&self, game: &Game) -> (Moves<Self::Move>, Position);
    fn compute_unmoves(&self, game: &Game) -> Moves<Self::Move>;
    fn apply_move(&self, game: &mut Game, move_: &Self::Move);
    fn unapply_move(&self, game: &mut Game, move_: &Self::Move);
    fn compute_new_frozen_boxes(
        &self,
        frozen: &Bitvector,
        game: &Game,
        box_idx: Index,
    ) -> Bitvector;
}

struct ForwardsSearchHelper {
    pi_corrals: bool,
}

struct BackwardsSearchHelper;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchType {
    Forward,
    Reverse,
    Bidirectional,
}

/// Manages iterative deepening A* by repeatedly calling Searcher with increasing thresholds
pub struct Solver<H: Heuristic, T: Tracer> {
    forward: Searcher<H, T, ForwardsSearchHelper>,
    reverse: Searcher<H, T, BackwardsSearchHelper>,
    search_type: SearchType,
}

pub trait Tracer {
    fn trace<M: Move>(
        &self,
        game: &Game,
        nodes_explored: usize,
        threshold: usize,
        f_cost: usize,
        g_cost: usize,
        move_: &M,
    );
}

#[derive(Debug, Copy, Clone)]
struct PushByPos {
    box_pos: Position,
    direction: Direction,
}

impl<H: Heuristic, T: Tracer, S: SearchHelper> Searcher<H, T, S> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        zobrist: Rc<Zobrist>,
        max_nodes_explored: usize,
        heuristic: H,
        initial_game: Game,
        initial_player_positions: Vec<Position>,
        freeze_deadlocks: bool,
        tracer: Option<Rc<T>>,
        helper: S,
    ) -> Self {
        let initial_boxes_hash = zobrist.compute_boxes_hash(&initial_game);
        let mut searcher = Searcher {
            nodes_explored: 0,
            max_nodes_explored,
            table: HashMap::new(),
            zobrist,
            heuristic,
            initial_game,
            initial_player_positions,
            initial_boxes_hash,
            freeze_deadlocks,
            // frozen_counts: HashMap::new(),
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
        self.reset();

        let mut result = SearchResult::Impossible;

        for pos in self.initial_player_positions.clone() {
            let mut game = self.initial_game.clone();
            game.set_player(pos);

            let mut frozen_boxes = if self.freeze_deadlocks {
                compute_frozen_boxes(&game)
            } else {
                Bitvector::new()
            };

            let search_result = self.search_helper(
                &mut game,
                &mut frozen_boxes,
                0,
                threshold,
                target_check,
                self.initial_boxes_hash,
                0,
            );

            match search_result {
                SearchResult::Solved(_) => return search_result,
                SearchResult::Cutoff => return search_result,
                SearchResult::Exceeded => result = SearchResult::Exceeded,
                SearchResult::Impossible => {}
            }
        }

        result
    }

    fn reset(&mut self) {
        self.table.clear();

        // Important: bidirectional search relies on the invariant that the
        // transposition table always contains the initial state (since when
        // we're doing a forward search we're checking the backward search's
        // table to see if we've finished, and vice versa).
        for &pos in &self.initial_player_positions {
            let initial_hash = self.initial_boxes_hash ^ self.zobrist.player_hash(pos);
            self.table.insert(
                initial_hash,
                TableEntry {
                    parent_hash: 0,
                    g_cost: 0,
                },
            );
        }
    }

    /// Perform DFS A* search up to the specified threshold
    /// Returns SearchResult. If found, use reconstruct_solution to get the solution path.
    #[allow(clippy::too_many_arguments)]
    fn search_helper<F>(
        &mut self,
        game: &mut Game,
        frozen_boxes: &mut Bitvector,
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
        match self.heuristic.compute(game) {
            Some(h_cost) => f_cost += h_cost as usize,
            None => return SearchResult::Impossible,
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
            let old_box_pos = game.box_position(move_.box_index());
            self.helper.apply_move(game, &move_);
            let new_box_pos = game.box_position(move_.box_index());

            if let Some(tracer) = &self.tracer {
                tracer.trace(game, self.nodes_explored, threshold, f_cost, g_cost, &move_);
            }

            // Compute frozen boxes
            let new_frozen =
                self.helper
                    .compute_new_frozen_boxes(frozen_boxes, game, move_.box_index());
            if game.unsolved_boxes().contains_any(&new_frozen) {
                // Deadlock
                self.helper.unapply_move(game, &move_);
                continue;
            }

            frozen_boxes.add_all(&new_frozen);

            // let mut frozen_hash = 0;
            // for box_idx in frozen_boxes.iter() {
            //     frozen_hash ^= self.zobrist.box_hash(game.box_position(box_idx));
            // }
            // *self.frozen_counts.entry(frozen_hash).or_default() += 1;

            // Update boxes hash (unhash old position, hash new position)
            let new_boxes_hash = boxes_hash
                ^ self.zobrist.box_hash(old_box_pos)
                ^ self.zobrist.box_hash(new_box_pos);

            let child_result = self.search_helper(
                game,
                frozen_boxes,
                g_cost + 1,
                threshold,
                target_check,
                new_boxes_hash,
                curr_hash,
            );
            if let SearchResult::Solved(_) = child_result {
                return child_result;
            }

            frozen_boxes.remove_all(&new_frozen);

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

    fn reconstruct_solution(&self, final_game: &Game, forward: bool) -> Vec<PushByPos> {
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
                let old_box_pos = current_game.box_position(unmove.box_index());
                self.helper.unapply_move(&mut current_game, &unmove);
                let new_box_pos = current_game.box_position(unmove.box_index());

                // Compute hash of this previous state
                let prev_hash = self.zobrist.compute_hash(&current_game);

                // Check if this matches the parent we're looking for
                if prev_hash == target_parent_hash {
                    solution.push(PushByPos {
                        box_pos: if forward { new_box_pos } else { old_box_pos },
                        direction: if forward {
                            unmove.direction()
                        } else {
                            unmove.direction().reverse()
                        },
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

        if forward {
            solution.reverse();
        }
        solution
    }
}

impl SearchHelper for ForwardsSearchHelper {
    type Move = Push;

    fn compute_moves(&self, game: &Game) -> (Moves<Push>, Position) {
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

    fn compute_new_frozen_boxes(
        &self,
        frozen: &Bitvector,
        game: &Game,
        box_idx: Index,
    ) -> Bitvector {
        compute_new_frozen_boxes(*frozen, game, box_idx)
    }
}

impl SearchHelper for BackwardsSearchHelper {
    type Move = Pull;

    fn compute_moves(&self, game: &Game) -> (Moves<Pull>, Position) {
        game.compute_pulls()
    }

    fn compute_unmoves(&self, game: &Game) -> Moves<Pull> {
        game.compute_pushes().0.to_pulls()
    }

    fn apply_move(&self, game: &mut Game, pull: &Pull) {
        game.pull(*pull);
    }

    fn unapply_move(&self, game: &mut Game, pull: &Pull) {
        game.push(pull.to_push())
    }

    fn compute_new_frozen_boxes(
        &self,
        _frozen: &Bitvector,
        _game: &Game,
        _box_idx: Index,
    ) -> Bitvector {
        Bitvector::new()
    }
}

impl<H: Heuristic, T: Tracer> Solver<H, T> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        max_nodes_explored: usize,
        forward_heuristic: H,
        reverse_heuristic: H,
        search_type: SearchType,
        game: Game,
        freeze_deadlocks: bool,
        pi_corrals: bool,
        tracer: Option<T>,
    ) -> Self {
        let zobrist = Rc::new(Zobrist::new());
        let tracer = tracer.map(|t| Rc::new(t));
        let goal_state = game.make_goal_state();
        let forward_player_positions = vec![game.canonical_player_pos()];
        let reverse_player_positions = goal_state.all_possible_player_positions();

        Solver {
            forward: Searcher::new(
                zobrist.clone(),
                max_nodes_explored,
                forward_heuristic,
                game,
                forward_player_positions,
                freeze_deadlocks,
                tracer.clone(),
                ForwardsSearchHelper { pi_corrals },
            ),
            reverse: Searcher::new(
                zobrist,
                max_nodes_explored,
                reverse_heuristic,
                goal_state,
                reverse_player_positions,
                false,
                tracer,
                BackwardsSearchHelper,
            ),
            search_type,
        }
    }

    pub fn nodes_explored(&self) -> (usize, usize) {
        (self.forward.nodes_explored(), self.reverse.nodes_explored())
    }

    // Implements bidirectional search. Note that this implementation does not
    // guarantee optimal solution paths in general. This is because the A*
    // search going from either end is not guaranteed to explore the states in
    // order of BFS distance. This means that when we find a solution (i.e.,
    // when the forwards and backwards searchers overlap), the combined solution
    // might not be optimal.
    pub fn solve(&mut self) -> SolveResult {
        let mut forward_threshold = 0;
        let mut reverse_threshold = 0;

        loop {
            let search_forward = match self.search_type {
                SearchType::Forward => true,
                SearchType::Reverse => false,
                SearchType::Bidirectional => {
                    self.forward.nodes_explored() <= self.reverse.nodes_explored()
                }
            };

            if search_forward {
                match self.forward.search(forward_threshold, &|hash| {
                    self.reverse.table.contains_key(&hash)
                }) {
                    SearchResult::Solved(game) => {
                        return SolveResult::Solved(self.reconstruct_solution(&game));
                    }
                    SearchResult::Exceeded => forward_threshold += 1,
                    SearchResult::Cutoff => return SolveResult::Cutoff,
                    SearchResult::Impossible => return SolveResult::Impossible,
                }
            } else {
                match self.reverse.search(reverse_threshold, &|hash| {
                    self.forward.table.contains_key(&hash)
                }) {
                    SearchResult::Solved(game) => {
                        return SolveResult::Solved(self.reconstruct_solution(&game));
                    }
                    SearchResult::Exceeded => reverse_threshold += 1,
                    SearchResult::Cutoff => return SolveResult::Cutoff,
                    SearchResult::Impossible => return SolveResult::Impossible,
                }
            }
        }
    }

    fn reconstruct_solution(&self, game: &Game) -> Vec<Push> {
        let forward_soln = self.forward.reconstruct_solution(game, true);
        let reverse_soln = self.reverse.reconstruct_solution(game, false);
        self.combine_solution(&forward_soln, &reverse_soln)
    }

    fn combine_solution(
        &self,
        forward_soln: &[PushByPos],
        reverse_soln: &[PushByPos],
    ) -> Vec<Push> {
        let mut test_game = self.forward.initial_game.clone();
        let mut soln = Vec::new();

        for (i, push_by_pos) in forward_soln.iter().chain(reverse_soln.iter()).enumerate() {
            // Get box index at this position
            let box_index = test_game.box_index(push_by_pos.box_pos).unwrap_or_else(|| {
                panic!(
                    "Solution verification failed: no box at position {} for push {}",
                    push_by_pos.box_pos,
                    i + 1
                )
            });

            // Compute valid pushes at this state
            let (valid_pushes, _canonical_pos) = test_game.compute_pushes();

            // Verify that this push is among the valid pushes
            let push = Push::new(box_index, push_by_pos.direction);
            assert!(
                valid_pushes.contains(push),
                "Solution verification failed: push {} (box at {}, direction {:?}) is not valid",
                i + 1,
                push_by_pos.box_pos,
                push_by_pos.direction
            );

            // Apply the push
            test_game.push(push);
            soln.push(push);
        }

        // Verify final state is solved
        assert!(
            test_game.is_solved(),
            "Solution verification failed: puzzle is not solved"
        );

        soln
    }
}

#[cfg(test)]
mod tests {
    use crate::heuristic::SimpleHeuristic;

    use super::*;

    #[test]
    fn test_solve_simple() {
        let game = parse_game(
            r#"
#####
#@$.#
#####
"#,
        );
        let mut solver = new_solver(game.clone());
        let result = solver.solve();

        assert!(matches!(result, SolveResult::Solved(_)));
        if let SolveResult::Solved(soln) = result {
            assert_eq!(soln.len(), 1);

            // Verify solution works
            let mut test_game = game.clone();
            for push in soln {
                test_game.push(push);
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
        let mut solver = new_solver(game);
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
        let mut solver = new_solver(game);
        let result = solver.solve();

        assert!(matches!(result, SolveResult::Solved(_)));
        if let SolveResult::Solved(soln) = result {
            assert_eq!(soln.len(), 2);

            // Verify solution works
            let mut test_game = Game::from_text(input).unwrap();
            for push in soln {
                test_game.push(push);
            }
            assert!(test_game.is_solved());
        }
    }

    #[test]
    fn test_solve_impossible() {
        let game = parse_game(
            r#"
#######
#@$ #.#
#######
"#,
        );
        let mut solver = new_solver(game);
        let result = solver.solve();
        assert_eq!(result, SolveResult::Impossible);
    }

    fn parse_game(text: &str) -> Game {
        Game::from_text(text.trim_matches('\n')).unwrap()
    }

    struct NullTracer {}

    impl Tracer for NullTracer {
        fn trace<M: Move>(
            &self,
            _game: &Game,
            _nodes_explored: usize,
            _threshold: usize,
            _f_cost: usize,
            _g_cost: usize,
            _move: &M,
        ) {
            unreachable!()
        }
    }

    fn new_solver(game: Game) -> Solver<SimpleHeuristic, NullTracer> {
        Solver::new(
            5000000,
            SimpleHeuristic::new_forward(&game),
            SimpleHeuristic::new_reverse(&game),
            SearchType::Forward,
            game,
            true,
            true,
            None,
        )
    }
}
