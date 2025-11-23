use arrayvec::ArrayVec;

use crate::{
    bits::{Bitvector, Index},
    game::{ALL_DIRECTIONS, Game, MAX_BOXES, MAX_SIZE, Position, Tile},
};
use std::collections::VecDeque;

/// Estimated cost returned by heuristic computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cost(u16);

impl Cost {
    pub const UNSOLVABLE: Cost = Cost(u16::MAX);
}

impl From<Cost> for usize {
    fn from(cost: Cost) -> usize {
        cost.0 as usize
    }
}

/// Trait for computing heuristics that estimate the number of moves (pushes/pulls) needed.
pub trait Heuristic {
    /// Create a push-oriented heuristic for forward search.
    fn new_push(game: &Game) -> Self
    where
        Self: Sized;

    /// Create a pull-oriented heuristic for reverse search.
    fn new_pull(game: &Game) -> Self
    where
        Self: Sized;

    /// Compute estimated number of moves (pushes/pulls).
    /// Returns UNSOLVABLE if the position is impossible to solve.
    fn compute(&self, game: &Game) -> Cost;
}

pub struct NullHeuristic;

impl Heuristic for NullHeuristic {
    fn new_push(_game: &Game) -> Self {
        NullHeuristic
    }

    fn new_pull(_game: &Game) -> Self {
        NullHeuristic
    }

    fn compute(&self, _game: &Game) -> Cost {
        Cost(0)
    }
}

/// A heuristic based on simple matching of boxes to goals using precomputed push/pull distances.
pub struct SimpleHeuristic {
    /// distances[idx][y][x] = minimum pushes/pulls to get a box from (x, y) to destination idx
    distances: Box<[[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES]>,
}

impl Heuristic for SimpleHeuristic {
    fn new_push(game: &Game) -> Self {
        let distances = Box::new(compute_push_distances(game));
        SimpleHeuristic { distances }
    }

    fn new_pull(game: &Game) -> Self {
        let distances = Box::new(compute_pull_distances(game));
        SimpleHeuristic { distances }
    }

    fn compute(&self, game: &Game) -> Cost {
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
                return Cost::UNSOLVABLE;
            }

            box_to_dst_total += box_to_dst;
        }

        let mut dst_to_box_total = 0;
        for &dist in dst_to_box.iter().take(box_count) {
            if dist == u16::MAX {
                return Cost::UNSOLVABLE;
            } else {
                dst_to_box_total += dist;
            }
        }

        Cost(std::cmp::max(dst_to_box_total, box_to_dst_total))
    }
}

/// Heuristic which attempts to match boxes and goals greedily to find a minimum
/// cost matching. Runs in O(n^2) rather than O(n^3) required by the optimal
/// approach.
pub struct GreedyHeuristic {
    /// distances[idx][y][x] = minimum pushes/pulls to get a box from (x, y) to destination idx
    distances: Box<[[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES]>,
}

impl Heuristic for GreedyHeuristic {
    fn new_push(game: &Game) -> Self {
        let distances = Box::new(compute_push_distances(game));
        GreedyHeuristic { distances }
    }

    fn new_pull(game: &Game) -> Self {
        let distances = Box::new(compute_pull_distances(game));
        GreedyHeuristic { distances }
    }

    fn compute(&self, game: &Game) -> Cost {
        const M: usize = MAX_BOXES * MAX_BOXES;
        const N: usize = MAX_SIZE * MAX_SIZE;
        let box_count = game.box_count();

        // Compute all pairs of distances between boxes <-> destinations
        let mut all_pairs: ArrayVec<(u16, Index, Index), M> = ArrayVec::new();
        for (box_idx, &pos) in game.box_positions().iter().enumerate() {
            let box_idx = Index(box_idx as u8);
            for dst_idx in 0..box_count {
                let distance = self.distances[dst_idx][pos.1 as usize][pos.0 as usize];
                if distance < u16::MAX {
                    let dst_idx = Index(dst_idx as u8);
                    all_pairs.push((distance, box_idx, dst_idx));
                }
            }
        }

        // Perform counting sort over distances (testing w/ built-in sorts
        // indicate they are too slow in comparison)
        counting_sort::<_, _, N>(&mut all_pairs, |&(distance, _, _)| distance as usize);

        // Walk through sorted pairs and start matching things up
        let mut total_distance = 0;
        let mut unmatched_boxes = Bitvector::full(box_count as u8);
        let mut unmatched_dsts = Bitvector::full(box_count as u8);
        for (distance, box_idx, dst_idx) in all_pairs {
            if unmatched_boxes.contains(box_idx) && unmatched_dsts.contains(dst_idx) {
                total_distance += distance;
                unmatched_boxes.remove(box_idx);
                unmatched_dsts.remove(dst_idx);
            }
        }

        // Compute distance lower bound for unmatched boxes -> goals
        let mut unmatched_box_to_dst = 0;
        for box_idx in unmatched_boxes.iter() {
            let pos = game.box_position(box_idx);
            let min_distance = (0..box_count)
                .map(|dst_idx| self.distances[dst_idx][pos.1 as usize][pos.0 as usize])
                .min()
                .unwrap();
            if min_distance == u16::MAX {
                return Cost::UNSOLVABLE;
            }
            unmatched_box_to_dst += min_distance;
        }

        // Compute distance lower bound for unmatched goals -> boxes
        let mut unmatched_dst_to_box = 0;
        for dst_idx in unmatched_dsts.iter() {
            let min_distance = game
                .box_positions()
                .iter()
                .map(|pos| self.distances[dst_idx.0 as usize][pos.1 as usize][pos.0 as usize])
                .min()
                .unwrap();
            if min_distance == u16::MAX {
                return Cost::UNSOLVABLE;
            }
            unmatched_dst_to_box += min_distance;
        }

        // Add distance for unmatched boxes <-> goals (pick whichever lower
        // bound is higher)
        total_distance += std::cmp::max(unmatched_box_to_dst, unmatched_dst_to_box);

        Cost(total_distance)
    }
}

/// Compute push distances from each goal to all positions using BFS with pulls
fn compute_push_distances(game: &Game) -> [[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES] {
    let mut distances = [[[u16::MAX; MAX_SIZE]; MAX_SIZE]; MAX_BOXES];

    for (goal_idx, &goal_pos) in game.goal_positions().iter().enumerate() {
        bfs_pulls(game, goal_pos, &mut distances[goal_idx]);
    }

    distances
}

/// Compute pull distances from each goal to all positions using BFS with pushes
fn compute_pull_distances(game: &Game) -> [[[u16; MAX_SIZE]; MAX_SIZE]; MAX_BOXES] {
    let mut distances = [[[u16::MAX; MAX_SIZE]; MAX_SIZE]; MAX_BOXES];

    for (goal_idx, &goal_pos) in game.goal_positions().iter().enumerate() {
        bfs_pushes(game, goal_pos, &mut distances[goal_idx]);
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

// This counting sort implementation assumes that all counts can fit in a u16!
fn counting_sort<T, F, const N: usize>(arr: &mut [T], key_fn: F)
where
    F: Fn(&T) -> usize,
{
    if arr.len() <= 1 {
        return;
    }

    // 1. Compute the maximum key
    let max_key = arr.iter().map(&key_fn).max().unwrap();

    let mut counts: ArrayVec<u16, N> = ArrayVec::new();
    let mut starts: ArrayVec<u16, N> = ArrayVec::new();
    let mut ends: ArrayVec<u16, N> = ArrayVec::new();
    for _ in 0..=max_key {
        counts.push(0);
        starts.push(0);
        ends.push(0);
    }

    // 2. Count frequencies
    // We use `max_key + 1` because the range is inclusive (0..=k)
    for item in arr.iter() {
        let key = key_fn(item);
        debug_assert!(counts[key] < u16::MAX);
        counts[key] += 1;
    }

    // 3. Calculate the starting write index for each key (Prefix Sums).
    // `starts[i]` tracks the next available slot for key `i`.
    // `ends[i]` tracks the boundary where bucket `i` ends.
    let mut current = 0;
    for i in 0..=max_key {
        starts[i] = current;
        current += counts[i];
        ends[i] = current;
    }

    // 4. Swap elements into their correct buckets
    // We iterate through each bucket `i`. While the current write position for bucket `i`
    // hasn't reached the end of the bucket, we check the element residing there.
    for i in 0..=max_key {
        while starts[i] < ends[i] {
            let current_key = key_fn(&arr[starts[i] as usize]);

            if current_key == i {
                // This element is already in the correct bucket.
                // Advance the write pointer for this bucket.
                starts[i] += 1;
            } else {
                // This element belongs to a different bucket (`current_key`).
                // Swap it to the next available slot in that target bucket.
                let dest = starts[current_key];
                arr.swap(starts[i] as usize, dest as usize);

                // Advance the write pointer for the target bucket.
                starts[current_key] += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    use super::*;

    #[test]
    fn test_simple_heuristic_solved() {
        let input = "####\n\
                     #@*#\n\
                     ####";
        let game = Game::from_text(input).unwrap();
        let heuristic = SimpleHeuristic::new_push(&game);

        assert_eq!(heuristic.compute(&game), Cost(0));
    }

    #[test]
    fn test_simple_heuristic_one_move() {
        let input = "####\n\
                     #@$.#\n\
                     ####";
        let game = Game::from_text(input).unwrap();
        let heuristic = SimpleHeuristic::new_push(&game);

        // Box at (2,1), goal at (3,1), push distance = 1
        assert_eq!(heuristic.compute(&game), Cost(1));
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
        let heuristic = SimpleHeuristic::new_push(&game);

        // Two boxes at (2,2) and (3,2), two goals at (2,3) and (3,3)
        // Simple matching should pair them optimally: each box is 1 away from a goal
        assert_eq!(heuristic.compute(&game), Cost(2));
    }

    #[test]
    fn test_counting_sort_random() {
        let mut rng = ChaCha8Rng::seed_from_u64(12345);

        for _ in 0..100 {
            let len = rng.gen_range(0..4096);
            let max_key = rng.gen_range(1..1024);
            let mut data: Vec<usize> = (0..len).map(|_| rng.gen_range(0..max_key)).collect();
            counting_sort::<_, _, 1024>(&mut data, |&x| x);
            assert!(data.is_sorted(), "Array not sorted: {:?}", data);
        }
    }
}
