use crate::game::{Game, Push};
use crate::zobrist::{TranspositionTable, Zobrist};

pub struct Solver {
    nodes_explored: usize,
    tpn_table: TranspositionTable,
    zobrist: Zobrist,
}

impl Solver {
    pub fn new() -> Self {
        Solver {
            nodes_explored: 0,
            tpn_table: TranspositionTable::new(),
            zobrist: Zobrist::new(),
        }
    }

    /// Solve the game using iterative deepening DFS
    pub fn solve(&mut self, game: &Game) -> Option<Vec<Push>> {
        // Check if already solved
        if game.is_solved() {
            return Some(Vec::new());
        }

        let mut solution = Vec::new();

        // Iterative deepening: try increasing depth limits
        for max_depth in 0..=100 {
            solution.clear();
            self.tpn_table.clear();

            // Initial hash: only hash box positions, not player
            let mut boxes_hash = 0u64;
            for box_idx in 0..game.box_count() {
                let (x, y) = game.box_position(box_idx);
                boxes_hash ^= self.zobrist.box_hash(x, y);
            }

            if self.dfs(&mut game.clone(), &mut solution, 0, max_depth, boxes_hash) {
                return Some(solution);
            }
        }

        None
    }

    pub fn nodes_explored(&self) -> usize {
        self.nodes_explored
    }

    fn dfs(
        &mut self,
        game: &mut Game,
        solution: &mut Vec<Push>,
        depth: usize,
        max_depth: usize,
        boxes_hash: u64,
    ) -> bool {
        self.nodes_explored += 1;

        // Check if solved
        if game.is_solved() {
            return true;
        }

        // Check depth limit
        if depth >= max_depth {
            return false;
        }

        // Get all valid pushes and canonical position
        let (pushes, canonical_pos) = game.compute_pushes();

        // Set player to canonical position
        game.set_player_pos(canonical_pos.0, canonical_pos.1);

        // Hash in the canonical player position
        let full_hash = boxes_hash ^ self.zobrist.player_hash(canonical_pos.0, canonical_pos.1);

        // Check transposition table
        if self.tpn_table.should_skip(full_hash, depth) {
            return false;
        }

        // Mark this state as visited
        self.tpn_table.insert(full_hash, depth);

        // Try each push
        for push in &pushes {
            let old_box_pos = game.box_position(push.box_index as usize);

            solution.push(push);
            game.push(push);

            let new_box_pos = game.box_position(push.box_index as usize);

            // Update boxes hash (unhash old position, hash new position)
            let new_boxes_hash = boxes_hash
                ^ self.zobrist.box_hash(old_box_pos.0, old_box_pos.1)
                ^ self.zobrist.box_hash(new_box_pos.0, new_box_pos.1);

            if self.dfs(game, solution, depth + 1, max_depth, new_boxes_hash) {
                return true;
            }

            game.unpush(push);
            solution.pop();
        }

        false
    }
}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
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

        let mut solver = Solver::new();
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

        let mut solver = Solver::new();
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

        let mut solver = Solver::new();
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
