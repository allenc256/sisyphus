use crate::game::{Game, Push};
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

/// Performs A* search up to a specified threshold
struct Searcher<H: Heuristic> {
    nodes_explored: usize,
    max_nodes_explored: usize,
    table: HashMap<u64, usize>, // Transposition table mapping state hash to g-cost of first visit
    zobrist: Zobrist,
    heuristic: H,
}

/// Manages iterative deepening A* by repeatedly calling Searcher with increasing thresholds
pub struct Solver<H: Heuristic> {
    searcher: Searcher<H>,
}

impl<H: Heuristic> Searcher<H> {
    fn new(max_nodes_explored: usize, heuristic: H) -> Self {
        Searcher {
            nodes_explored: 0,
            max_nodes_explored,
            table: HashMap::new(),
            zobrist: Zobrist::new(),
            heuristic,
        }
    }

    fn nodes_explored(&self) -> usize {
        self.nodes_explored
    }

    fn reset(&mut self) {
        self.table.clear();
    }

    /// Perform DFS A* search up to the specified threshold
    /// Returns SearchResult and modifies solution in place if found
    fn search(
        &mut self,
        game: &mut Game,
        solution: &mut Vec<Push>,
        g_cost: usize,
        threshold: usize,
        boxes_hash: u64,
    ) -> SearchResult {
        self.nodes_explored += 1;

        // Check if we've exceeded the node limit
        if self.nodes_explored > self.max_nodes_explored {
            return SearchResult::Cutoff;
        }

        // Compute heuristic and f-cost
        let h_cost = self.heuristic.compute_forward(game);
        let f_cost = g_cost + h_cost;

        // If f-cost exceeds threshold, stop searching this branch
        if f_cost > threshold {
            return SearchResult::Exceeded;
        }

        // Check if solved
        if game.is_solved() {
            return SearchResult::Found;
        }

        // Get all valid pushes and canonical position
        let (pushes, canonical_pos) = game.compute_pushes();

        // Hash in the canonical player position
        let full_hash = boxes_hash ^ self.zobrist.player_hash(canonical_pos.0, canonical_pos.1);

        // Check transposition table
        if let Some(&prev_g_cost) = self.table.get(&full_hash) {
            // Skip if we've seen this state at a shallower or equal g-cost
            if g_cost >= prev_g_cost {
                return SearchResult::Exceeded;
            }
        }

        // Mark this state as visited
        self.table.insert(full_hash, g_cost);

        // Try each push
        for push in &pushes {
            let old_box_pos = game.box_pos(push.box_index as usize);

            solution.push(push);
            game.push(push);

            let new_box_pos = game.box_pos(push.box_index as usize);

            // Update boxes hash (unhash old position, hash new position)
            let new_boxes_hash = boxes_hash
                ^ self.zobrist.box_hash(old_box_pos.0, old_box_pos.1)
                ^ self.zobrist.box_hash(new_box_pos.0, new_box_pos.1);

            let result = self.search(game, solution, g_cost + 1, threshold, new_boxes_hash);
            if result == SearchResult::Found {
                return SearchResult::Found;
            }

            game.unpush(push);
            solution.pop();

            if result == SearchResult::Cutoff {
                return SearchResult::Cutoff;
            }
        }

        SearchResult::Exceeded
    }
}

impl<H: Heuristic> Solver<H> {
    pub fn new(max_nodes_explored: usize, heuristic: H) -> Self {
        Solver {
            searcher: Searcher::new(max_nodes_explored, heuristic),
        }
    }

    /// Solve the game using IDA* (Iterative Deepening A*)
    pub fn solve(&mut self, game: &Game) -> Option<Vec<Push>> {
        // Check if already solved
        if game.is_solved() {
            return Some(Vec::new());
        }

        let mut solution = Vec::new();

        // Initial hash: only hash box positions, not player
        let mut boxes_hash = 0u64;
        for box_idx in 0..game.box_count() {
            let (x, y) = game.box_pos(box_idx);
            boxes_hash ^= self.searcher.zobrist.box_hash(x, y);
        }

        // IDA*: try increasing f-cost thresholds
        let mut threshold = self.searcher.heuristic.compute_forward(game);

        loop {
            solution.clear();
            self.searcher.reset();

            match self
                .searcher
                .search(&mut game.clone(), &mut solution, 0, threshold, boxes_hash)
            {
                SearchResult::Found => {
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
