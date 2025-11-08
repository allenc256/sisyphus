use crate::game::{Game, Push};

pub struct Solver {
    nodes_explored: usize,
}

impl Solver {
    pub fn new() -> Self {
        Solver { nodes_explored: 0 }
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

            if self.dfs(&mut game.clone(), &mut solution, 0, max_depth) {
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

        // Try each push
        for push in &pushes {
            solution.push(push);
            game.push(push);

            if self.dfs(game, solution, depth + 1, max_depth) {
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
