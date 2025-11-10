use crate::game::{Game, PlayerPos, Push, Pushes};
use crate::heuristic::Heuristic;
use crate::zobrist::Zobrist;
use std::collections::HashMap;

/// Result of a search iteration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchResult {
    /// Solution found
    Found,
    /// No solution at this threshold
    Exceeded,
    /// Node limit exceeded
    Cutoff,
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
    zobrist: Zobrist,
    heuristic: H,
    direction: SearchDirection,
}

enum SearchDirection {
    Forwards,
    Backwards,
}

/// Manages iterative deepening A* by repeatedly calling Searcher with increasing thresholds
pub struct Solver<H: Heuristic> {
    searcher: Searcher<H>,
}

impl<H: Heuristic> Searcher<H> {
    fn new(max_nodes_explored: usize, heuristic: H, direction: SearchDirection) -> Self {
        Searcher {
            nodes_explored: 0,
            max_nodes_explored,
            table: HashMap::new(),
            zobrist: Zobrist::new(),
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
        let mut boxes_hash = 0u64;
        for box_idx in 0..game.box_count() {
            let (x, y) = game.box_pos(box_idx);
            boxes_hash ^= self.zobrist.box_hash(x, y);
        }
        let canonical_pos = game.canonical_player_pos();
        boxes_hash ^ self.zobrist.player_hash(canonical_pos)
    }

    /// Reconstruct solution by following parent_hash links backwards from final state
    /// Panics if solution reconstruction fails
    fn reconstruct_solution(&self, final_game: &Game) -> Vec<Push> {
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
                self.unapply_move(&mut current_game, unmove);

                // Compute hash of this previous state
                let prev_hash = self.compute_hash(&current_game);

                // Check if this matches the parent we're looking for
                if prev_hash == target_parent_hash {
                    // This is the correct unpush (which was a push in forward direction)
                    solution.push(unmove);
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

        // Reverse solution since we built it backwards
        solution.reverse();
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
    pub fn new(max_nodes_explored: usize, heuristic: H) -> Self {
        Solver {
            searcher: Searcher::new(max_nodes_explored, heuristic, SearchDirection::Forwards),
        }
    }

    /// Solve the game using IDA* (Iterative Deepening A*)
    pub fn solve(&mut self, game: &Game) -> Option<Vec<Push>> {
        // Check if already solved
        if game.is_solved() {
            return Some(Vec::new());
        }

        let target_hash = self.compute_target_hash(game);

        // Initial hash: only hash box positions, not player
        let mut boxes_hash = 0u64;
        for box_idx in 0..game.box_count() {
            let (x, y) = game.box_pos(box_idx);
            boxes_hash ^= self.searcher.zobrist.box_hash(x, y);
        }

        // IDA*: try increasing f-cost thresholds
        let mut threshold = self.searcher.compute_heuristic(game);

        loop {
            self.searcher.reset();
            let mut search_game = game.clone();

            match self
                .searcher
                .search(&mut search_game, 0, threshold, target_hash, boxes_hash, 0)
            {
                SearchResult::Found => {
                    let solution = self.searcher.reconstruct_solution(&search_game);
                    self.verify_solution(game, &solution);
                    return Some(solution);
                }
                SearchResult::Exceeded => {
                    threshold += 1;
                }
                SearchResult::Cutoff => {
                    return None;
                }
            }

            // If we've exceeded max nodes, give up
            if self.searcher.nodes_explored > self.searcher.max_nodes_explored {
                return None;
            }
        }
    }

    fn compute_target_hash(&self, game: &Game) -> u64 {
        match self.searcher.direction {
            SearchDirection::Forwards => {
                let mut target = game.clone();
                target.set_to_goal_state();
                self.searcher.compute_hash(&target)
            }
            SearchDirection::Backwards => self.searcher.compute_hash(game),
        }
    }

    pub fn nodes_explored(&self) -> usize {
        self.searcher.nodes_explored()
    }

    fn verify_solution(&self, game: &Game, solution: &[Push]) {
        let mut test_game = game.clone();
        for (i, push) in solution.iter().enumerate() {
            // Compute valid pushes at this state
            let (valid_pushes, _canonical_pos) = test_game.compute_pushes();

            // Verify that this push is among the valid pushes
            assert!(
                valid_pushes.contains(*push),
                "Solution verification failed: push {} (box {}, direction {:?}) is not valid",
                i + 1,
                push.box_index,
                push.direction
            );

            // Apply the push
            test_game.push(*push);
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
        Self::new(5000000, crate::heuristic::GreedyHeuristic::new())
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
        let mut solver = Solver::new(5000000, heuristic);
        let solution = solver.solve(&game);

        assert!(solution.is_some());
        let moves = solution.unwrap();
        assert_eq!(moves.len(), 1);

        // Verify solution works
        let mut test_game = Game::from_text(input).unwrap();
        for push in moves {
            test_game.push(push);
        }
        assert!(test_game.is_solved());
    }

    #[test]
    fn test_solve_already_solved() {
        let input = "####\n\
                     #@*#\n\
                     ####";
        let game = Game::from_text(input).unwrap();

        let heuristic = crate::heuristic::GreedyHeuristic::new();
        let mut solver = Solver::new(5000000, heuristic);
        let solution = solver.solve(&game);

        assert!(solution.is_some());
        assert_eq!(solution.unwrap().len(), 0);
    }

    #[test]
    fn test_solve_two_moves() {
        let input = "#####\n\
                     #@$ .#\n\
                     #####";
        let game = Game::from_text(input).unwrap();

        let heuristic = crate::heuristic::GreedyHeuristic::new();
        let mut solver = Solver::new(5000000, heuristic);
        let solution = solver.solve(&game);

        assert!(solution.is_some());
        let moves = solution.unwrap();
        assert_eq!(moves.len(), 2);

        // Verify solution works
        let mut test_game = Game::from_text(input).unwrap();
        for push in moves {
            test_game.push(push);
        }
        assert!(test_game.is_solved());
    }
}
