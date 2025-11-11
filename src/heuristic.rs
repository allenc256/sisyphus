use crate::game::{Game, MAX_BOXES};

/// Trait for computing heuristics that estimate the number of pushes needed to solve a game.
pub trait Heuristic: Clone {
    /// Compute estimated number of pushes needed to complete the game from the current state.
    fn compute_forward(&self, game: &Game) -> usize;

    /// Compute estimated number of pushes needed to get to the current state from the initial state.
    fn compute_backward(&self, game: &Game) -> usize;
}

#[derive(Clone)]
pub struct NullHeuristic;

impl NullHeuristic {
    pub fn new() -> Self {
        NullHeuristic
    }
}

impl Heuristic for NullHeuristic {
    fn compute_forward(&self, _game: &Game) -> usize {
        0
    }

    fn compute_backward(&self, _game: &Game) -> usize {
        0
    }
}

/// A heuristic based on greedy matching of boxes to goals using Manhattan distance.
/// This heuristic is not admissible, so using it may produce sub-optimal solutions.
#[derive(Clone)]
pub struct GreedyHeuristic;

impl GreedyHeuristic {
    pub fn new() -> Self {
        GreedyHeuristic
    }

    fn manhattan_distance(pos1: (u8, u8), pos2: (u8, u8)) -> usize {
        let dx = (pos1.0 as i32 - pos2.0 as i32).abs();
        let dy = (pos1.1 as i32 - pos2.1 as i32).abs();
        (dx + dy) as usize
    }

    fn greedy_distance(
        src_positions: &mut [(u8, u8); MAX_BOXES],
        dst_positions: &mut [(u8, u8); MAX_BOXES],
        unmatched_count: usize,
    ) -> usize {
        let mut total_distance = 0;
        let mut unmatched_count = unmatched_count;

        // Greedy matching: repeatedly find and match the closest src-dst pair
        #[allow(clippy::needless_range_loop)]
        while unmatched_count > 0 {
            let mut min_distance = usize::MAX;
            let mut best_src_idx = 0;
            let mut best_dst_idx = 0;

            // Find the closest unmatched src-dst pair
            for src_i in 0..unmatched_count {
                let src_pos = src_positions[src_i];

                for dst_i in 0..unmatched_count {
                    let dst_pos = dst_positions[dst_i];
                    let distance = Self::manhattan_distance(src_pos, dst_pos);

                    if distance < min_distance {
                        min_distance = distance;
                        best_src_idx = src_i;
                        best_dst_idx = dst_i;
                    }
                }
            }

            // Add distance to total
            total_distance += min_distance;

            // Remove matched box and goal using swap with last element
            unmatched_count -= 1;
            src_positions[best_src_idx] = src_positions[unmatched_count];
            dst_positions[best_dst_idx] = dst_positions[unmatched_count];
        }

        total_distance
    }
}

impl Heuristic for GreedyHeuristic {
    fn compute_forward(&self, game: &Game) -> usize {
        let box_count = game.box_count();
        let mut boxes = [(0u8, 0u8); MAX_BOXES];
        let mut goals = [(0u8, 0u8); MAX_BOXES];

        for i in 0..box_count {
            boxes[i] = game.box_pos(i);
            goals[i] = game.goal_pos(i);
        }

        Self::greedy_distance(&mut boxes, &mut goals, box_count)
    }

    fn compute_backward(&self, game: &Game) -> usize {
        let box_count = game.box_count();
        let mut box_starts = [(0u8, 0u8); MAX_BOXES];
        let mut boxes = [(0u8, 0u8); MAX_BOXES];

        for i in 0..box_count {
            box_starts[i] = game.box_start_pos(i);
            boxes[i] = game.box_pos(i);
        }

        Self::greedy_distance(&mut box_starts, &mut boxes, box_count)
    }
}

impl Default for GreedyHeuristic {
    fn default() -> Self {
        Self::new()
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
        let heuristic = GreedyHeuristic::new();

        assert_eq!(heuristic.compute_forward(&game), 0);
    }

    #[test]
    fn test_greedy_heuristic_one_move() {
        let input = "####\n\
                     #@$.#\n\
                     ####";
        let game = Game::from_text(input).unwrap();
        let heuristic = GreedyHeuristic::new();

        // Box at (2,1), goal at (3,1), Manhattan distance = 1
        assert_eq!(heuristic.compute_forward(&game), 1);
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
        let heuristic = GreedyHeuristic::new();

        // Two boxes at (2,2) and (3,2), two goals at (2,3) and (3,3)
        // Greedy matching should pair them optimally: each box is 1 away from a goal
        assert_eq!(heuristic.compute_forward(&game), 2);
    }
}
