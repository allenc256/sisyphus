use crate::game::{Game, MAX_BOXES};

/// Trait for computing heuristics that estimate the number of pushes needed to solve a game.
pub trait Heuristic {
    /// Compute the estimated number of pushes needed to complete the game from the current state.
    /// Returns 0 if the game is already solved.
    /// A good heuristic should be:
    /// - Admissible: never overestimate the actual number of pushes needed
    /// - Consistent: h(n) <= cost(n, n') + h(n') for any successor n' of n
    fn compute(&self, game: &Game) -> usize;
}

pub struct NullHeuristic;

impl NullHeuristic {
    pub fn new() -> Self {
        NullHeuristic
    }
}

impl Heuristic for NullHeuristic {
    fn compute(&self, _game: &Game) -> usize {
        0
    }
}

/// A heuristic based on greedy matching of boxes to goals using Manhattan distance.
/// This heuristic is not admissible, so using it may produce sub-optimal solutions.
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
}

impl Heuristic for GreedyHeuristic {
    fn compute(&self, game: &Game) -> usize {
        let box_count = game.box_count();
        let goal_count = game.goal_count();

        if box_count == 0 || goal_count == 0 {
            return 0;
        }

        // Arrays to track unmatched boxes and goals
        // Initialize with indices 0, 1, 2, ...
        let mut boxes_left = [0; MAX_BOXES];
        let mut goals_left = [0; MAX_BOXES];
        let mut boxes_left_count = box_count;
        let mut goals_left_count = goal_count;

        for i in 0..box_count {
            boxes_left[i] = i;
            goals_left[i] = i;
        }

        let mut total_distance = 0;

        // Greedy matching: repeatedly find and match the closest box-goal pair
        #[allow(clippy::needless_range_loop)]
        while boxes_left_count > 0 && goals_left_count > 0 {
            let mut min_distance = usize::MAX;
            let mut best_box_idx = 0;
            let mut best_goal_idx = 0;

            // Find the closest unmatched box-goal pair
            for box_i in 0..boxes_left_count {
                let box_idx = boxes_left[box_i];
                let box_pos = game.box_pos(box_idx);

                for goal_i in 0..goals_left_count {
                    let goal_idx = goals_left[goal_i];
                    let goal_pos = game.goal_pos(goal_idx);
                    let distance = Self::manhattan_distance(box_pos, goal_pos);

                    if distance < min_distance {
                        min_distance = distance;
                        best_box_idx = box_i;
                        best_goal_idx = goal_i;
                    }
                }
            }

            // Add distance to total
            total_distance += min_distance;

            // Remove matched box and goal using swap with last element
            boxes_left_count -= 1;
            boxes_left[best_box_idx] = boxes_left[boxes_left_count];

            goals_left_count -= 1;
            goals_left[best_goal_idx] = goals_left[goals_left_count];
        }

        total_distance
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

        assert_eq!(heuristic.compute(&game), 0);
    }

    #[test]
    fn test_greedy_heuristic_one_move() {
        let input = "####\n\
                     #@$.#\n\
                     ####";
        let game = Game::from_text(input).unwrap();
        let heuristic = GreedyHeuristic::new();

        // Box at (2,1), goal at (3,1), Manhattan distance = 1
        assert_eq!(heuristic.compute(&game), 1);
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
        assert_eq!(heuristic.compute(&game), 2);
    }
}
