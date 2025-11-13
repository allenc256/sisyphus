use crate::game::{ALL_DIRECTIONS, Game, MAX_BOXES, MAX_SIZE, Tile};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cost {
    Solvable(u16),
    Impossible,
}

/// Trait for computing heuristics that estimate the number of pushes needed to solve a game.
pub trait Heuristic {
    /// Compute estimated number of pushes needed to complete the game from the current state.
    fn compute_forward(&self, game: &Game) -> Cost;

    /// Compute estimated number of pushes needed to get to the current state from the initial state.
    fn compute_backward(&self, game: &Game) -> Cost;
}

pub struct NullHeuristic;

impl NullHeuristic {
    pub fn new() -> Self {
        NullHeuristic
    }
}

impl Heuristic for NullHeuristic {
    fn compute_forward(&self, _game: &Game) -> Cost {
        Cost::Solvable(0)
    }

    fn compute_backward(&self, _game: &Game) -> Cost {
        Cost::Solvable(0)
    }
}

/// A heuristic based on greedy matching of boxes to goals using precomputed push distances.
pub struct GreedyHeuristic {
    /// goal_distances[goal_idx][y][x] = minimum pushes to get a box from (x, y) to goal goal_idx
    goal_distances: [[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES],
    /// start_distances[box_idx][y][x] = minimum pulls to get a box from (x, y) to start position box_idx
    start_distances: [[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES],
}

impl GreedyHeuristic {
    pub fn new(game: &Game) -> Self {
        let goal_distances = Self::compute_distances_from_goals(game);
        let start_distances = Self::compute_distances_from_starts(game);
        GreedyHeuristic {
            goal_distances,
            start_distances,
        }
    }

    /// Compute push distances from each goal to all positions using BFS with pulls
    fn compute_distances_from_goals(game: &Game) -> [[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES] {
        let mut distances = [[[u16::MAX; MAX_SIZE]; MAX_SIZE]; MAX_BOXES];

        for goal_idx in 0..game.box_count() {
            Self::bfs_pulls(game, goal_idx, &mut distances[goal_idx]);
        }

        distances
    }

    /// Compute pull distances from each start position to all positions using BFS with pushes
    fn compute_distances_from_starts(game: &Game) -> [[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES] {
        let mut distances = [[[u16::MAX; MAX_SIZE]; MAX_SIZE]; MAX_BOXES];

        for box_idx in 0..game.box_count() {
            Self::bfs_pushes(game, box_idx, &mut distances[box_idx]);
        }

        distances
    }

    /// BFS using pulls to compute distances from a goal position
    fn bfs_pulls(game: &Game, goal_idx: usize, distances: &mut [[u16; MAX_SIZE]; MAX_SIZE]) {
        let start_pos = game.goal_pos(goal_idx);
        let mut queue = VecDeque::new();
        queue.push_back((start_pos.0, start_pos.1));
        distances[start_pos.1 as usize][start_pos.0 as usize] = 0;

        while let Some((box_x, box_y)) = queue.pop_front() {
            let dist = distances[box_y as usize][box_x as usize];

            for direction in ALL_DIRECTIONS {
                if let Some((new_box_x, new_box_y)) = game.pull_pos(box_x, box_y, direction) {
                    if let Some((player_x, player_y)) =
                        game.pull_pos(new_box_x, new_box_y, direction)
                    {
                        let new_box_tile = game.get_tile(new_box_x, new_box_y);
                        let player_tile = game.get_tile(player_x, player_y);

                        if (new_box_tile == Tile::Floor || new_box_tile == Tile::Goal)
                            && (player_tile == Tile::Floor || player_tile == Tile::Goal)
                            && distances[new_box_y as usize][new_box_x as usize] == u16::MAX
                        {
                            distances[new_box_y as usize][new_box_x as usize] = dist + 1;
                            queue.push_back((new_box_x, new_box_y));
                        }
                    }
                }
            }
        }
    }

    /// BFS using pushes to compute distances from a box start position
    fn bfs_pushes(game: &Game, box_idx: usize, distances: &mut [[u16; MAX_SIZE]; MAX_SIZE]) {
        let start_pos = game.box_start_pos(box_idx);
        let mut queue = VecDeque::new();
        queue.push_back((start_pos.0, start_pos.1));
        distances[start_pos.1 as usize][start_pos.0 as usize] = 0;

        while let Some((box_x, box_y)) = queue.pop_front() {
            let dist = distances[box_y as usize][box_x as usize];

            for direction in ALL_DIRECTIONS {
                if let Some((new_box_x, new_box_y)) = game.push_pos(box_x, box_y, direction) {
                    if let Some((player_x, player_y)) = game.pull_pos(box_x, box_y, direction) {
                        let new_box_tile = game.get_tile(new_box_x, new_box_y);
                        let player_tile = game.get_tile(player_x, player_y);

                        if (new_box_tile == Tile::Floor || new_box_tile == Tile::Goal)
                            && (player_tile == Tile::Floor || player_tile == Tile::Goal)
                            && distances[new_box_y as usize][new_box_x as usize] == u16::MAX
                        {
                            distances[new_box_y as usize][new_box_x as usize] = dist + 1;
                            queue.push_back((new_box_x, new_box_y));
                        }
                    }
                }
            }
        }
    }

    fn greedy_distance(game: &Game, distances: &[[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES]) -> Cost {
        let mut total_distance = 0u16;
        let box_count = game.box_count();

        // For each box, find the closest destination
        for i in 0..box_count {
            let pos = game.box_pos(i);
            let mut min_distance = u16::MAX;

            // Find the minimum distance to any destination
            for dst_idx in 0..box_count {
                let distance = distances[dst_idx][pos.1 as usize][pos.0 as usize];
                if distance < min_distance {
                    min_distance = distance;
                }
            }

            if min_distance == u16::MAX {
                return Cost::Impossible;
            }

            total_distance += min_distance;
        }

        Cost::Solvable(total_distance)
    }
}

impl Heuristic for GreedyHeuristic {
    fn compute_forward(&self, game: &Game) -> Cost {
        Self::greedy_distance(game, &self.goal_distances)
    }

    fn compute_backward(&self, game: &Game) -> Cost {
        Self::greedy_distance(game, &self.start_distances)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greedy_heuristic_solved() {
        let input = "####\n\
                     #@*#\n\
                     ####";
        let game = Game::from_text(input).unwrap();
        let heuristic = GreedyHeuristic::new(&game);

        assert_eq!(heuristic.compute_forward(&game), Cost::Solvable(0));
    }

    #[test]
    fn test_greedy_heuristic_one_move() {
        let input = "####\n\
                     #@$.#\n\
                     ####";
        let game = Game::from_text(input).unwrap();
        let heuristic = GreedyHeuristic::new(&game);

        // Box at (2,1), goal at (3,1), push distance = 1
        assert_eq!(heuristic.compute_forward(&game), Cost::Solvable(1));
    }

    #[test]
    fn test_greedy_heuristic_multiple_boxes() {
        let input = "######\n\
                     #    #\n\
                     # $$ #\n\
                     # .. #\n\
                     #  @ #\n\
                     ######";
        let game = Game::from_text(input).unwrap();
        let heuristic = GreedyHeuristic::new(&game);

        // Two boxes at (2,2) and (3,2), two goals at (2,3) and (3,3)
        // Greedy matching should pair them optimally: each box is 1 away from a goal
        assert_eq!(heuristic.compute_forward(&game), Cost::Solvable(2));
    }
}
