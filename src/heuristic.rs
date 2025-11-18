use crate::{
    bits::Bitvector,
    game::{ALL_DIRECTIONS, Game, MAX_BOXES, MAX_SIZE, Tile},
};
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

pub struct SimpleHeuristic {
    /// goal_distances[goal_idx][y][x] = minimum pushes to get a box from (x, y) to goal goal_idx
    goal_distances: Box<[[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES]>,
    /// start_distances[box_idx][y][x] = minimum pulls to get a box from (x, y) to start position box_idx
    start_distances: Box<[[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES]>,
}

impl SimpleHeuristic {
    pub fn new(game: &Game) -> Self {
        let goal_distances = Box::new(compute_distances_from_goals(game));
        let start_distances = Box::new(compute_distances_from_starts(game));
        Self {
            goal_distances,
            start_distances,
        }
    }

    fn simple_distance(game: &Game, distances: &[[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES]) -> Cost {
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

impl Heuristic for SimpleHeuristic {
    fn compute_forward(&self, game: &Game) -> Cost {
        Self::simple_distance(game, &self.goal_distances)
    }

    fn compute_backward(&self, game: &Game) -> Cost {
        Self::simple_distance(game, &self.start_distances)
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
    pub fn new(game: &Game) -> Self {
        let goal_distances = Box::new(compute_distances_from_goals(game));
        let start_distances = Box::new(compute_distances_from_starts(game));
        GreedyHeuristic {
            goal_distances,
            start_distances,
        }
    }

    fn greedy_distance(game: &Game, distances: &[[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES]) -> Cost {
        let box_count = game.box_count();
        let mut matched_dsts = Bitvector::new();
        let mut matched_boxes = Bitvector::new();

        // First eliminate all boxes and goals that have already been matched.
        for box_idx in 0..box_count {
            let box_pos = game.box_pos(box_idx as usize);
            for dst_idx in 0..box_count {
                if distances[dst_idx][box_pos.1 as usize][box_pos.0 as usize] == 0 {
                    matched_dsts.add(dst_idx as u8);
                    matched_boxes.add(box_idx as u8);
                    break;
                }
            }
        }

        let mut total_distance = 0;

        for dst_idx in 0..box_count {
            if matched_dsts.contains(dst_idx as u8) {
                continue;
            }

            let mut best_dist = u16::MAX;
            let mut best_unmatched_box_idx = usize::MAX;
            let mut best_unmatched_dist = u16::MAX;

            for box_idx in 0..box_count {
                let box_pos = game.box_pos(box_idx);
                let distance = distances[dst_idx][box_pos.1 as usize][box_pos.0 as usize];

                best_dist = std::cmp::min(best_dist, distance);

                if distance < best_unmatched_dist && !matched_boxes.contains(box_idx as u8) {
                    best_unmatched_dist = distance;
                    best_unmatched_box_idx = box_idx;
                }
            }

            if best_unmatched_dist < u16::MAX {
                total_distance += best_unmatched_dist;
                matched_boxes.add(best_unmatched_box_idx as u8);
            } else if best_dist < u16::MAX {
                total_distance += best_dist;
            } else {
                return Cost::Impossible;
            }
        }

        Cost::Solvable(total_distance)
    }

    // fn greedy_distance(game: &Game, distances: &[[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES]) -> Cost {
    //     fn pack_bits(distance: u16, box_idx: usize, dst_idx: usize) -> u32 {
    //         ((distance as u32) << 16) | ((box_idx as u32) << 8) | (dst_idx as u32)
    //     }

    //     fn unpack_bits(bits: u32) -> (u16, u8, u8) {
    //         let distance = (bits >> 16) as u16;
    //         let box_idx = (bits >> 8) as u8;
    //         let dst_idx = bits as u8;
    //         (distance, box_idx, dst_idx)
    //     }

    //     let mut matches: ArrayVec<u32, { MAX_BOXES * MAX_BOXES }> = ArrayVec::new();
    //     let mut matched_boxes = Bitvector::new();
    //     let mut matched_dsts = Bitvector::new();
    //     let box_count = game.box_count();

    //     for box_idx in 0..box_count {
    //         let pos = game.box_pos(box_idx);
    //         let mut skip = false;

    //         for dst_idx in 0..box_count {
    //             let distance = distances[dst_idx][pos.1 as usize][pos.0 as usize];
    //             if distance == 0 {
    //                 skip = true;
    //                 matched_boxes.add(box_idx as u8);
    //                 matched_dsts.add(dst_idx as u8);
    //                 break;
    //             }
    //             if distance != u16::MAX {
    //                 matches.push(pack_bits(distance, box_idx, dst_idx));
    //             }
    //         }

    //         if skip {
    //             continue;
    //         }
    //     }

    //     matches.sort_unstable();

    //     let mut total_distance = 0;
    //     for &bits in matches.iter() {
    //         let (distance, box_idx, dst_idx) = unpack_bits(bits);
    //         if matched_boxes.contains(box_idx) || matched_dsts.contains(dst_idx) {
    //             continue;
    //         }
    //         matched_boxes.add(box_idx);
    //         matched_dsts.add(dst_idx);
    //         total_distance += distance;
    //     }

    //     for box_idx in 0..box_count {
    //         if !matched_boxes.contains(box_idx as u8) {
    //             let pos = game.box_pos(box_idx);
    //             let mut min_distance = u16::MAX;

    //             for dst_idx in 0..box_idx {
    //                 let distance = distances[dst_idx][pos.1 as usize][pos.0 as usize];
    //                 min_distance = std::cmp::min(min_distance, distance);
    //             }

    //             if min_distance == u16::MAX {
    //                 return Cost::Impossible;
    //             } else {
    //                 total_distance += min_distance;
    //             }
    //         }
    //     }

    //     Cost::Solvable(total_distance)
    // }
}

impl Heuristic for GreedyHeuristic {
    fn compute_forward(&self, game: &Game) -> Cost {
        Self::greedy_distance(game, &self.goal_distances)
    }

    fn compute_backward(&self, game: &Game) -> Cost {
        Self::greedy_distance(game, &self.start_distances)
    }
}

/// Compute push distances from each goal to all positions using BFS with pulls
fn compute_distances_from_goals(game: &Game) -> [[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES] {
    let mut distances = [[[u16::MAX; MAX_SIZE]; MAX_SIZE]; MAX_BOXES];

    for goal_idx in 0..game.box_count() {
        bfs_pulls(game, goal_idx, &mut distances[goal_idx]);
    }

    distances
}

/// Compute pull distances from each start position to all positions using BFS with pushes
fn compute_distances_from_starts(game: &Game) -> [[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES] {
    let mut distances = [[[u16::MAX; MAX_SIZE]; MAX_SIZE]; MAX_BOXES];

    for box_idx in 0..game.box_count() {
        bfs_pushes(game, box_idx, &mut distances[box_idx]);
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
                if let Some((player_x, player_y)) = game.pull_pos(new_box_x, new_box_y, direction) {
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

    //     #[test]
    //     fn test_greedy_heuristic_1() {
    //         let game = parse_game(
    //             r#"
    //  #########
    // ##   #   ##
    // #    #$   #
    // #   $#   $#
    // #   **.   #
    // ####+ .####
    // #  $...   #
    // #    #$   #
    // #    #    #
    // ##   # $ ##
    //  #########
    // "#,
    //         );

    //         let greedy = GreedyHeuristic::new(&game);
    //         println!("h_cost={:?}:\n{}", greedy.compute_forward(&game), game);
    //     }

    // fn parse_game(text: &str) -> Game {
    //     Game::from_text(text.trim_matches('\n')).unwrap()
    // }
}
