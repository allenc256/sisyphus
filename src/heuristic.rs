use crate::game::{ALL_DIRECTIONS, Game, MAX_BOXES, MAX_SIZE, Position, Tile};
use std::collections::VecDeque;

/// Trait for computing heuristics that estimate the number of moves (pushes/pulls) needed.
/// Returns None if the position is impossible to solve, or Some(cost) with the estimated cost.
pub trait Heuristic {
    /// Compute estimated number of moves (pushes/pulls).
    fn compute(&self, game: &Game) -> Option<u16>;
}

pub struct NullHeuristic;

impl NullHeuristic {
    pub fn new() -> Self {
        NullHeuristic
    }
}

impl Heuristic for NullHeuristic {
    fn compute(&self, _game: &Game) -> Option<u16> {
        Some(0)
    }
}

/// A heuristic based on simple matching of boxes to goals using precomputed push/pull distances.
pub struct SimpleHeuristic {
    /// distances[idx][y][x] = minimum pushes/pulls to get a box from (x, y) to destination idx
    distances: Box<[[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES]>,
}

impl SimpleHeuristic {
    pub fn new_forward(game: &Game) -> Self {
        let distances = Box::new(compute_distances_from_goals(game));
        SimpleHeuristic { distances }
    }

    pub fn new_reverse(game: &Game) -> Self {
        let distances = Box::new(compute_distances_from_starts(game));
        SimpleHeuristic { distances }
    }
}

impl Heuristic for SimpleHeuristic {
    fn compute(&self, game: &Game) -> Option<u16> {
        // Compute two distances:
        //   box_to_dst_total: total distance from each box to its nearest destination.
        //   dst_to_box_total: total distance from each destination to its nearest box.
        // The simple distance is the maximum between the two.
        // If either distance is u16::MAX, then the game is unsolvable.

        let mut box_to_dst_total = 0u16;
        let mut dst_to_box = [u16::MAX; MAX_BOXES];
        let box_count = game.box_count();

        for pos in game.box_positions().iter() {
            let mut box_to_dst = u16::MAX;

            for (dst_idx, dst_to_box) in dst_to_box.iter_mut().enumerate().take(box_count) {
                let distance = self.distances[dst_idx][pos.1 as usize][pos.0 as usize];
                box_to_dst = std::cmp::min(box_to_dst, distance);
                *dst_to_box = std::cmp::min(*dst_to_box, distance);
            }

            if box_to_dst == u16::MAX {
                return None;
            }

            box_to_dst_total += box_to_dst;
        }

        let mut dst_to_box_total = 0;
        for &dist in dst_to_box.iter().take(box_count) {
            if dist == u16::MAX {
                return None;
            } else {
                dst_to_box_total += dist;
            }
        }

        Some(std::cmp::max(dst_to_box_total, box_to_dst_total))
    }
}

/// Compute push distances from each goal to all positions using BFS with pulls
fn compute_distances_from_goals(game: &Game) -> [[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES] {
    let mut distances = [[[u16::MAX; MAX_SIZE]; MAX_SIZE]; MAX_BOXES];

    for (goal_idx, &goal_pos) in game.goal_positions().iter().enumerate() {
        bfs_pulls(game, goal_pos, &mut distances[goal_idx]);
    }

    distances
}

/// Compute pull distances from each start position to all positions using BFS with pushes
fn compute_distances_from_starts(game: &Game) -> [[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES] {
    let mut distances = [[[u16::MAX; MAX_SIZE]; MAX_SIZE]; MAX_BOXES];

    for (box_idx, &start_pos) in game.start_positions().iter().enumerate() {
        bfs_pushes(game, start_pos, &mut distances[box_idx]);
    }

    distances
}

/// BFS using pulls to compute distances from a goal position
fn bfs_pulls(game: &Game, goal_pos: Position, distances: &mut [[u16; MAX_SIZE]; MAX_SIZE]) {
    let mut queue = VecDeque::new();
    queue.push_back(goal_pos);
    distances[goal_pos.1 as usize][goal_pos.0 as usize] = 0;

    while let Some(box_pos) = queue.pop_front() {
        let dist = distances[box_pos.1 as usize][box_pos.0 as usize];

        for direction in ALL_DIRECTIONS {
            if let Some(new_box_pos) = game.move_position(box_pos, direction.reverse()) {
                if let Some(player_pos) = game.move_position(new_box_pos, direction.reverse()) {
                    let new_box_tile = game.get_tile(new_box_pos);
                    let player_tile = game.get_tile(player_pos);

                    if (new_box_tile == Tile::Floor || new_box_tile == Tile::Goal)
                        && (player_tile == Tile::Floor || player_tile == Tile::Goal)
                        && distances[new_box_pos.1 as usize][new_box_pos.0 as usize] == u16::MAX
                    {
                        distances[new_box_pos.1 as usize][new_box_pos.0 as usize] = dist + 1;
                        queue.push_back(new_box_pos);
                    }
                }
            }
        }
    }
}

/// BFS using pushes to compute distances from a box start position
fn bfs_pushes(game: &Game, start_pos: Position, distances: &mut [[u16; MAX_SIZE]; MAX_SIZE]) {
    let mut queue = VecDeque::new();
    queue.push_back(start_pos);
    distances[start_pos.1 as usize][start_pos.0 as usize] = 0;

    while let Some(box_pos) = queue.pop_front() {
        let dist = distances[box_pos.1 as usize][box_pos.0 as usize];

        for direction in ALL_DIRECTIONS {
            if let Some(new_box_pos) = game.move_position(box_pos, direction) {
                if let Some(player_pos) = game.move_position(box_pos, direction.reverse()) {
                    let new_box_tile = game.get_tile(new_box_pos);
                    let player_tile = game.get_tile(player_pos);

                    if (new_box_tile == Tile::Floor || new_box_tile == Tile::Goal)
                        && (player_tile == Tile::Floor || player_tile == Tile::Goal)
                        && distances[new_box_pos.1 as usize][new_box_pos.0 as usize] == u16::MAX
                    {
                        distances[new_box_pos.1 as usize][new_box_pos.0 as usize] = dist + 1;
                        queue.push_back(new_box_pos);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_heuristic_solved() {
        let input = "####\n\
                     #@*#\n\
                     ####";
        let game = Game::from_text(input).unwrap();
        let heuristic = SimpleHeuristic::new_forward(&game);

        assert_eq!(heuristic.compute(&game), Some(0));
    }

    #[test]
    fn test_simple_heuristic_one_move() {
        let input = "####\n\
                     #@$.#\n\
                     ####";
        let game = Game::from_text(input).unwrap();
        let heuristic = SimpleHeuristic::new_forward(&game);

        // Box at (2,1), goal at (3,1), push distance = 1
        assert_eq!(heuristic.compute(&game), Some(1));
    }

    #[test]
    fn test_simple_heuristic_multiple_boxes() {
        let input = "######\n\
                     #    #\n\
                     # $$ #\n\
                     # .. #\n\
                     #  @ #\n\
                     ######";
        let game = Game::from_text(input).unwrap();
        let heuristic = SimpleHeuristic::new_forward(&game);

        // Two boxes at (2,2) and (3,2), two goals at (2,3) and (3,3)
        // Simple matching should pair them optimally: each box is 1 away from a goal
        assert_eq!(heuristic.compute(&game), Some(2));
    }
}
