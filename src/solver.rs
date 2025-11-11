use crate::game::{Game, PlayerPos, Push, PushByPos, Pushes};
use crate::heuristic::Heuristic;
use crate::zobrist::Zobrist;
use std::collections::HashMap;
use std::rc::Rc;

/// Result of a search iteration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchResult {
    /// Solution found
    Found,
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
    Solved(Vec<PushByPos>),
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
struct Searcher<H: Heuristic> {
    nodes_explored: usize,
    max_nodes_explored: usize,
    table: HashMap<u64, TableEntry>, // Transposition table mapping state hash to entry
    zobrist: Rc<Zobrist>,
    heuristic: H,
    direction: SearchDirection,
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
pub struct Solver<H: Heuristic> {
    forwards: Searcher<H>,
    backwards: Searcher<H>,
    search_type: SearchType,
}

impl<H: Heuristic> Searcher<H> {
    fn new(
        zobrist: Rc<Zobrist>,
        max_nodes_explored: usize,
        heuristic: H,
        direction: SearchDirection,
    ) -> Self {
        Searcher {
            nodes_explored: 0,
            max_nodes_explored,
            table: HashMap::new(),
            zobrist,
            heuristic,
            direction,
        }
    }

    fn nodes_explored(&self) -> usize {
        self.nodes_explored
    }

    fn reset(&mut self) {
        self.table.clear();
    }

    /// Compute the hash for a game state (boxes hash XOR canonical player position hash)
    fn compute_hash(&self, game: &Game) -> u64 {
        let boxes_hash = self.compute_boxes_hash(game);
        let canonical_pos = game.canonical_player_pos();
        boxes_hash ^ self.zobrist.player_hash(canonical_pos)
    }

    fn compute_boxes_hash(&self, game: &Game) -> u64 {
        let mut boxes_hash = 0u64;
        for box_idx in 0..game.box_count() {
            let (x, y) = game.box_pos(box_idx);
            boxes_hash ^= self.zobrist.box_hash(x, y);
        }
        boxes_hash
    }

    /// Reconstruct solution by following parent_hash links backwards from final state
    /// Panics if solution reconstruction fails
    /// Returns solution as position-based pushes (PushByPos)
    fn reconstruct_solution(&self, final_game: &Game) -> Vec<PushByPos> {
        let mut solution = Vec::new();
        let mut current_game = final_game.clone();
        let mut current_hash = self.compute_hash(&current_game);

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

            // Compute all possible unpushes from current state
            let (unmoves, _canonical_pos) = self.compute_unmoves(&current_game);

            // Try each unpush to find which one leads to parent state
            let mut found = false;
            for unmove in &unmoves {
                let old_box_pos = current_game.box_pos(unmove.box_index as usize);
                self.unapply_move(&mut current_game, unmove);
                let new_box_pos = current_game.box_pos(unmove.box_index as usize);

                // Compute hash of this previous state
                let prev_hash = self.compute_hash(&current_game);

                // Check if this matches the parent we're looking for
                if prev_hash == target_parent_hash {
                    solution.push(PushByPos {
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

                // Redo the unpush if it wasn't correct
                self.apply_move(&mut current_game, unmove);
            }

            assert!(
                found,
                "Failed to reconstruct solution: no unpush leads to parent state"
            );
        }

        if self.direction == SearchDirection::Forwards {
            // Reverse solution since we built it backwards
            solution.reverse();
        }

        solution
    }

    /// Perform DFS A* search up to the specified threshold
    /// Returns SearchResult. If found, use reconstruct_solution to get the solution path.
    fn search(
        &mut self,
        game: &mut Game,
        g_cost: usize,
        threshold: usize,
        target_hash: u64,
        boxes_hash: u64,
        parent_hash: u64,
    ) -> SearchResult {
        self.nodes_explored += 1;

        // Check if we've exceeded the node limit
        if self.nodes_explored > self.max_nodes_explored {
            return SearchResult::Cutoff;
        }

        // Compute heuristic and f-cost
        let h_cost = self.compute_heuristic(game);
        let f_cost = g_cost + h_cost;

        // If f-cost exceeds threshold, stop searching this branch
        if f_cost > threshold {
            return SearchResult::Exceeded;
        }

        // Get all valid pushes and canonical position
        let (pushes, canonical_pos) = self.compute_moves(game);

        // Hash in the canonical player position
        let curr_hash = boxes_hash ^ self.zobrist.player_hash(canonical_pos);

        // Check transposition table
        if let Some(entry) = self.table.get(&curr_hash) {
            // Skip if we've seen this state at a shallower or equal g-cost
            if g_cost >= entry.g_cost {
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
        if curr_hash == target_hash {
            return SearchResult::Found;
        }

        // Try each push
        for push in &pushes {
            let old_box_pos = game.box_pos(push.box_index as usize);
            self.apply_move(game, push);
            let new_box_pos = game.box_pos(push.box_index as usize);

            // Update boxes hash (unhash old position, hash new position)
            let new_boxes_hash = boxes_hash
                ^ self.zobrist.box_hash(old_box_pos.0, old_box_pos.1)
                ^ self.zobrist.box_hash(new_box_pos.0, new_box_pos.1);

            let result = self.search(
                game,
                g_cost + 1,
                threshold,
                target_hash,
                new_boxes_hash,
                curr_hash,
            );
            if result == SearchResult::Found {
                return result;
            }

            self.unapply_move(game, push);

            if result == SearchResult::Cutoff {
                return SearchResult::Cutoff;
            }

            if result == SearchResult::Impossible {
                return SearchResult::Impossible;
            }
        }

        SearchResult::Exceeded
    }

    fn compute_heuristic(&self, game: &Game) -> usize {
        match self.direction {
            SearchDirection::Forwards => self.heuristic.compute_forward(game),
            SearchDirection::Backwards => self.heuristic.compute_backward(game),
        }
    }

    fn compute_moves(&self, game: &Game) -> (Pushes, PlayerPos) {
        match self.direction {
            SearchDirection::Forwards => game.compute_pushes(),
            SearchDirection::Backwards => game.compute_unpushes(),
        }
    }

    fn compute_unmoves(&self, game: &Game) -> (Pushes, PlayerPos) {
        match self.direction {
            SearchDirection::Forwards => game.compute_unpushes(),
            SearchDirection::Backwards => game.compute_pushes(),
        }
    }

    fn apply_move(&self, game: &mut Game, push: Push) {
        match self.direction {
            SearchDirection::Forwards => game.push(push),
            SearchDirection::Backwards => game.unpush(push),
        }
    }

    fn unapply_move(&self, game: &mut Game, push: Push) {
        match self.direction {
            SearchDirection::Forwards => game.unpush(push),
            SearchDirection::Backwards => game.push(push),
        }
    }
}

impl<H: Heuristic> Solver<H> {
    pub fn new(max_nodes_explored: usize, heuristic: H, search_type: SearchType) -> Self {
        let zobrist = Rc::new(Zobrist::new());
        Solver {
            forwards: Searcher::new(
                zobrist.clone(),
                max_nodes_explored,
                heuristic.clone(),
                SearchDirection::Forwards,
            ),
            backwards: Searcher::new(
                zobrist,
                max_nodes_explored,
                heuristic,
                SearchDirection::Backwards,
            ),
            search_type,
        }
    }

    /// Solve the game using IDA* (Iterative Deepening A*)
    pub fn solve(&mut self, game: &Game) -> SolveResult {
        match self.search_type {
            SearchType::Forwards => self.solve_helper(SearchDirection::Forwards, game),
            SearchType::Backwards => self.solve_helper(SearchDirection::Backwards, game),
            SearchType::Bidirectional => todo!(),
        }
    }

    fn solve_helper(&mut self, direction: SearchDirection, game: &Game) -> SolveResult {
        // Check if already solved
        if game.is_solved() {
            return SolveResult::Solved(Vec::new());
        }

        let searcher = match direction {
            SearchDirection::Forwards => &mut self.forwards,
            SearchDirection::Backwards => &mut self.backwards,
        };

        let mut initial_game = game.clone();
        let mut target_game = game.clone();

        match direction {
            SearchDirection::Forwards => target_game.set_to_goal_state(),
            SearchDirection::Backwards => initial_game.set_to_goal_state(),
        }

        let target_hash = searcher.compute_hash(&target_game);
        let boxes_hash = searcher.compute_boxes_hash(&initial_game);

        // IDA*: try increasing f-cost thresholds
        let mut threshold = searcher.compute_heuristic(game);

        loop {
            searcher.reset();
            let mut search_game = initial_game.clone();

            match searcher.search(&mut search_game, 0, threshold, target_hash, boxes_hash, 0) {
                SearchResult::Found => {
                    let solution = searcher.reconstruct_solution(&search_game);
                    self.verify_solution(game, &solution);
                    return SolveResult::Solved(solution);
                }
                SearchResult::Exceeded => {
                    threshold += 1;
                }
                SearchResult::Cutoff => {
                    return SolveResult::Cutoff;
                }
                SearchResult::Impossible => {
                    return SolveResult::Impossible;
                }
            }
        }
    }

    pub fn nodes_explored(&self) -> usize {
        self.forwards.nodes_explored() + self.backwards.nodes_explored()
    }

    fn verify_solution(&self, game: &Game, solution: &[PushByPos]) {
        let mut test_game = game.clone();
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
            let index_push = Push {
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

impl Default for Solver<crate::heuristic::GreedyHeuristic> {
    fn default() -> Self {
        Self::new(
            5000000,
            crate::heuristic::GreedyHeuristic::new(),
            SearchType::Forwards,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solve_simple() {
        let input = "####\n\
                     #@$.#\n\
                     ####";
        let game = Game::from_text(input).unwrap();

        let heuristic = crate::heuristic::GreedyHeuristic::new();
        let mut solver = Solver::new(5000000, heuristic, SearchType::Forwards);
        let result = solver.solve(&game);

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

        let heuristic = crate::heuristic::GreedyHeuristic::new();
        let mut solver = Solver::new(5000000, heuristic, SearchType::Forwards);
        let result = solver.solve(&game);

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

        let heuristic = crate::heuristic::GreedyHeuristic::new();
        let mut solver = Solver::new(5000000, heuristic, SearchType::Forwards);
        let result = solver.solve(&game);

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
}
