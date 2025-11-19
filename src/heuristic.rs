use crate::game::{ALL_DIRECTIONS, Game, GameType, MAX_BOXES, MAX_SIZE, Tile};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cost {
    Solvable(u16),
    Impossible,
}

/// Trait for computing heuristics that estimate the number of pushes needed to solve a game.
pub trait Heuristic {
    /// Compute estimated number of pushes needed to complete the game from the current state.
    fn compute_forward<T: GameType>(&self, game: &Game<T>) -> Cost;

    /// Compute estimated number of pushes needed to get to the current state from the initial state.
    fn compute_backward<T: GameType>(&self, game: &Game<T>) -> Cost;
}

pub struct NullHeuristic;

impl NullHeuristic {
    pub fn new() -> Self {
        NullHeuristic
    }
}

impl Heuristic for NullHeuristic {
    fn compute_forward<T: GameType>(&self, _game: &Game<T>) -> Cost {
        Cost::Solvable(0)
    }

    fn compute_backward<T: GameType>(&self, _game: &Game<T>) -> Cost {
        Cost::Solvable(0)
    }
}

/// A heuristic based on greedy matching of boxes to goals using precomputed push distances.
pub struct GreedyHeuristic {
    /// goal_distances[goal_idx][y][x] = minimum pushes to get a box from (x, y) to goal goal_idx
    goal_distances: Box<[[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES]>,
    /// start_distances[box_idx][y][x] = minimum pulls to get a box from (x, y) to start position box_idx
    start_distances: Box<[[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES]>,
}

#[allow(clippy::needless_range_loop)]
impl GreedyHeuristic {
    pub fn new<T: GameType>(game: &Game<T>) -> Self {
        let goal_distances = Box::new(Self::compute_distances_from_goals(game));
        let start_distances = Box::new(Self::compute_distances_from_starts(game));
        GreedyHeuristic {
            goal_distances,
            start_distances,
        }
    }

    /// Compute push distances from each goal to all positions using BFS with pulls
    fn compute_distances_from_goals<T: GameType>(
        game: &Game<T>,
    ) -> [[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES] {
        let mut distances = [[[u16::MAX; MAX_SIZE]; MAX_SIZE]; MAX_BOXES];

        for goal_idx in 0..game.box_count() {
            Self::bfs_pulls(game, goal_idx, &mut distances[goal_idx]);
        }

        distances
    }

    /// Compute pull distances from each start position to all positions using BFS with pushes
    fn compute_distances_from_starts<T: GameType>(
        game: &Game<T>,
    ) -> [[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES] {
        let mut distances = [[[u16::MAX; MAX_SIZE]; MAX_SIZE]; MAX_BOXES];

        for box_idx in 0..game.box_count() {
            Self::bfs_pushes(game, box_idx, &mut distances[box_idx]);
        }

        distances
    }

    /// BFS using pulls to compute distances from a goal position
    fn bfs_pulls<T: GameType>(
        game: &Game<T>,
        goal_idx: usize,
        distances: &mut [[u16; MAX_SIZE]; MAX_SIZE],
    ) {
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
    fn bfs_pushes<T: GameType>(
        game: &Game<T>,
        box_idx: usize,
        distances: &mut [[u16; MAX_SIZE]; MAX_SIZE],
    ) {
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

    fn greedy_distance<T: GameType>(
        game: &Game<T>,
        distances: &[[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES],
    ) -> Cost {
        // Compute two distances:
        //   box_to_dst_total: total distance from each box to its nearest destination.
        //   dst_to_box_total: total distance from each destination to its nearest box.
        // The greedy distance is the maximum between the two.
        // If either distance is u16::MAX, then the game is unsolvable.

        let mut box_to_dst_total = 0u16;
        let mut dst_to_box = [u16::MAX; MAX_BOXES];
        let box_count = game.box_count();

        for i in 0..box_count {
            let pos = game.box_pos(i);
            let mut box_to_dst = u16::MAX;

            for dst_idx in 0..box_count {
                let distance = distances[dst_idx][pos.1 as usize][pos.0 as usize];
                box_to_dst = std::cmp::min(box_to_dst, distance);
                dst_to_box[dst_idx] = std::cmp::min(dst_to_box[dst_idx], distance);
            }

            if box_to_dst == u16::MAX {
                return Cost::Impossible;
            }

            box_to_dst_total += box_to_dst;
        }

        let mut dst_to_box_total = 0;
        for i in 0..box_count {
            let dist = dst_to_box[i];
            if dist == u16::MAX {
                return Cost::Impossible;
            } else {
                dst_to_box_total += dist;
            }
        }

        Cost::Solvable(std::cmp::max(dst_to_box_total, box_to_dst_total))
    }
}

impl Heuristic for GreedyHeuristic {
    fn compute_forward<T: GameType>(&self, game: &Game<T>) -> Cost {
        Self::greedy_distance(game, &self.goal_distances)
    }

    fn compute_backward<T: GameType>(&self, game: &Game<T>) -> Cost {
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
