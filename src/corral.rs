use std::collections::HashMap;
use std::rc::Rc;

use arrayvec::ArrayVec;

use crate::{
    bits::{Bitvector, LazyBitboard, Position},
    game::{ALL_DIRECTIONS, Game, MAX_SIZE, Move, Moves, Push, ReachableSet, Tile},
    zobrist::Zobrist,
};

struct Corral {
    /// The boxes in the corral, including boxes on the edge of the corral.
    boxes: Bitvector,
    /// The extent of the corral. This includes all boxes within the corral,
    /// including its edge.
    extent: LazyBitboard,
    /// Valid corral pushes.
    pushes: Moves<Push>,
    i_condition: bool,
    p_condition: bool,
}

fn compute_corral(game: &Game, pos: Position, reachable: &ReachableSet<Push>) -> Option<Corral> {
    assert!(!reachable.squares.get(pos));

    let mut stack: ArrayVec<Position, { MAX_SIZE * MAX_SIZE }> = ArrayVec::new();
    let mut extent = LazyBitboard::new();
    let mut boxes = Bitvector::new();
    let mut boxes_on_edge = Bitvector::new();
    let mut requires_push = false;

    // Start DFS from the given position
    stack.push(pos);
    extent.set(pos);

    // Perform DFS to find full extent of corral
    while let Some(curr_pos) = stack.pop() {
        let is_goal = game.get_tile(curr_pos) == Tile::Goal;

        // We've hit a box
        if let Some(box_idx) = game.box_index(curr_pos) {
            boxes.add(box_idx);
            // Box not on goal: corral requires pushes to solve the puzzle
            if !is_goal {
                requires_push = true;
            }
            // If we've hit the edge of the corral, stop exploring further
            if reachable.boxes.contains(box_idx) {
                boxes_on_edge.add(box_idx);
                continue;
            }
        } else if is_goal {
            // Goal without a box: corral requires pushes to solve the puzzle
            requires_push = true;
        }

        // Otherwise, continue searching in all directions
        for &dir in &ALL_DIRECTIONS {
            if let Some(next_pos) = game.move_position(curr_pos, dir) {
                if game.get_tile(next_pos) != Tile::Wall && !extent.get(next_pos) {
                    stack.push(next_pos);
                    extent.set(next_pos);
                }
            }
        }
    }

    if !requires_push {
        return None;
    }

    let mut i_condition = true;
    let mut p_condition = true;
    let mut pushes = Moves::new();

    for box_idx in boxes_on_edge {
        let box_pos = game.box_position(box_idx);
        for &dir in &ALL_DIRECTIONS {
            if let (Some(next_pos), Some(player_pos)) = (
                game.move_position(box_pos, dir),
                game.move_position(box_pos, dir.reverse()),
            ) {
                // Ignore pushes originating from within the corral
                if extent.get(player_pos) {
                    continue;
                }
                // Ignore pushes into a wall or box
                if game.get_tile(next_pos) == Tile::Wall || game.box_index(next_pos).is_some() {
                    continue;
                }
                // Ignore pushes coming from a wall
                if game.get_tile(player_pos) == Tile::Wall {
                    continue;
                }
                // Ignore pushes into dead squares
                if game.is_push_dead_square(next_pos) {
                    continue;
                }
                // Check I condition: the push must lead into the corral
                if !extent.get(next_pos) {
                    i_condition = false;
                    continue;
                }
                // Check P condition: the player must be capable of making the push
                if !reachable.squares.get(player_pos) {
                    p_condition = false;
                    continue;
                }
                // Record inward player push
                pushes.add(box_idx, dir);
            }
        }
    }

    Some(Corral {
        boxes,
        extent,
        pushes,
        i_condition,
        p_condition,
    })
}

pub struct CorralSearcher {
    deadlocks: DeadlockSearcher,
}

impl CorralSearcher {
    pub fn new(zobrist: Rc<Zobrist>, max_nodes_explored: usize) -> Self {
        Self {
            deadlocks: DeadlockSearcher::new(zobrist, max_nodes_explored),
        }
    }

    pub fn search(
        &mut self,
        game: &mut Game,
        reachable: &ReachableSet<Push>,
    ) -> CorralResult<Push> {
        let mut result = CorralResult::None;
        let mut min_cost = usize::MAX;
        let mut visited = LazyBitboard::new();

        for push in &reachable.moves {
            let box_pos = game.box_position(push.box_index());
            let new_pos = game.move_position(box_pos, push.direction()).unwrap();
            // Look for a corral by examining the other side of a push.
            if !reachable.squares.get(new_pos) && !visited.get(new_pos) {
                if let Some(corral) = compute_corral(game, new_pos, reachable) {
                    visited.set_all(&corral.extent);
                    if corral.i_condition {
                        // Check for corral deadlocks
                        if self.deadlocks.search(game, &corral) == DeadlockResult::Deadlocked {
                            return CorralResult::Deadlocked;
                        }

                        // This is PI-corral, so it is eligible for pruning
                        if corral.p_condition {
                            let cost = corral.pushes.len();
                            if cost < min_cost {
                                result = CorralResult::Prune(corral.pushes);
                                min_cost = cost;
                            }
                        }
                    }
                }
            }
        }

        result
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorralResult<T> {
    Prune(Moves<T>),
    Deadlocked,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeadlockResult {
    Ok,
    Deadlocked,
    CutOff,
}

struct DeadlockSearcher {
    /// Transposition table which contains search results for corrals.
    corral_table: HashMap<u64, DeadlockResult>,
    /// Transposition table which is cleared and reused on each search.
    search_table: HashMap<u64, usize>,
    zobrist: Rc<Zobrist>,
    max_nodes_explored: usize,
}

impl DeadlockSearcher {
    fn new(zobrist: Rc<Zobrist>, max_nodes_explored: usize) -> Self {
        Self {
            corral_table: HashMap::new(),
            search_table: HashMap::new(),
            zobrist,
            max_nodes_explored,
        }
    }

    /// Search for corral deadlocks.
    fn search(&mut self, game: &mut Game, corral: &Corral) -> DeadlockResult {
        if self.max_nodes_explored == 0 {
            return DeadlockResult::Ok;
        }

        // Project the game down to only boxes within the corral
        let checkpoint = game.checkpoint();
        game.project(corral.boxes);

        // Clear the working transposition table
        self.search_table.clear();

        // Perform the search
        let mut nodes_explored = 0;
        let partial_hash = self.zobrist.compute_boxes_hash(game);
        let result = self.search_helper(game, corral, 0, &mut nodes_explored, partial_hash);

        // Undo projection
        game.restore(&checkpoint);

        result
    }

    fn search_helper(
        &mut self,
        game: &mut Game,
        corral: &Corral,
        depth: usize,
        nodes_explored: &mut usize,
        partial_hash: u64,
    ) -> DeadlockResult {
        *nodes_explored += 1;

        // Check if the game is solved (all boxes on goals)
        if game.is_solved() {
            return DeadlockResult::Ok;
        }

        // Compute all possible pushes
        let reachable = game.compute_pushes();

        // Compute full state hash (boxes + canonical player position)
        let canonical_player_pos = reachable.squares.top_left().unwrap();
        let hash = partial_hash ^ self.zobrist.player_hash(canonical_player_pos);

        // Check corral transposition table
        if let Some(&prev_result) = self.corral_table.get(&hash) {
            return prev_result;
        }

        // Check search transposition table
        if let Some(&prev_result) = self.search_table.get(&hash) {
            // Skip if we've seen this state at a shallower or equal depth
            if depth >= prev_result {
                return DeadlockResult::Deadlocked;
            }
        }

        // Mark this state as visited at this depth
        self.search_table.insert(hash, depth);

        // Check if we're allowed to explore children
        if *nodes_explored >= self.max_nodes_explored {
            return DeadlockResult::CutOff;
        }

        let mut result = DeadlockResult::Deadlocked;

        // Try each push
        for push in &reachable.moves {
            // Get the old and new box positions
            let old_box_pos = game.box_position(push.box_index());
            let new_box_pos = game.move_position(old_box_pos, push.direction()).unwrap();

            // Prune dead square pushes
            if game.is_push_dead_square(new_box_pos) {
                continue;
            }

            // Check if the box would be pushed out of the corral
            if !corral.extent.get(new_box_pos) {
                result = DeadlockResult::Ok;
                break;
            }

            // Make the push
            game.push(push);

            // Update partial hash incrementally (unhash old box position, hash
            // new box position)
            let partial_hash = partial_hash
                ^ self.zobrist.box_hash(old_box_pos)
                ^ self.zobrist.box_hash(new_box_pos);

            // Recursively search
            let child_result =
                self.search_helper(game, corral, depth + 1, nodes_explored, partial_hash);

            // Undo the push
            game.pull(push.to_pull());

            // Stop immediately in the following cases
            if child_result == DeadlockResult::Ok || child_result == DeadlockResult::CutOff {
                result = child_result;
                break;
            }
        }

        // Update the corral table if at root
        if depth == 0 {
            self.corral_table.insert(hash, result);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::{bits::Index, game::Direction};

    use super::*;

    #[test]
    fn test_pi_corral_1() {
        let game = parse_game(
            r#"
########
#  $  .#
#   $@.#
#  $  .#
####   #
   # $.#
   #####
"#,
        );

        let corral = compute_corral_helper(&game, 3, 2);
        assert!(!corral.i_condition);
        assert!(corral.p_condition);
    }

    #[test]
    fn test_pi_corral_2() {
        let game = parse_game(
            r#"
########
#  $  .#
#   $@.#
#  $# .#
####   #
   # $.#
   #####
"#,
        );

        let mut pushes = Moves::new();
        pushes.add(Index(0), Direction::Left);
        pushes.add(Index(1), Direction::Left);

        let corral = compute_corral_helper(&game, 3, 2);
        assert!(corral.i_condition);
        assert!(corral.p_condition);
        assert_eq!(corral.pushes, pushes);
    }

    #[test]
    fn test_pi_corral_3() {
        let game = parse_game(
            r#"
########
#.$.$ .#
#.  $@$#
#. $   #
####   #
   #   #
   #####
"#,
        );

        let mut pushes = Moves::new();
        pushes.add(Index(1), Direction::Left);
        pushes.add(Index(2), Direction::Left);
        pushes.add(Index(4), Direction::Left);

        let corral = compute_corral_helper(&game, 3, 2);
        assert!(corral.i_condition);
        assert!(corral.p_condition);
        assert_eq!(corral.pushes, pushes);
    }

    #[test]
    fn test_pi_corral_4() {
        let game = parse_game(
            r#"
########
#.  $ .#
#. $@ $#
#. $$  #
####   #
   #  .#
   #####
"#,
        );

        let corral = compute_corral_helper(&game, 2, 2);
        assert!(!corral.i_condition);
        assert!(corral.p_condition);
    }

    #[test]
    fn test_pi_corral_5() {
        let game = parse_game(
            r#"
########
#.  $ .#
#. $@ $#
#. $#  #
####   #
   #   #
   #####
"#,
        );

        let mut expected_moves = Moves::new();
        expected_moves.add(Index(0), Direction::Left);
        expected_moves.add(Index(1), Direction::Left);

        let corral = compute_corral_helper(&game, 2, 2);
        assert!(corral.i_condition);
        assert!(corral.p_condition);
        assert_eq!(corral.pushes, expected_moves);
    }

    #[test]
    fn test_pi_corral_6() {
        let game = parse_game(
            r#"
##########
#   #    #
#.  $ @$.#
####$$####
  #    #
  # .. #
  ######
"#,
        );

        let mut pushes = Moves::new();
        pushes.add(Index(0), Direction::Left);

        let corral1 = compute_corral_helper(&game, 3, 2);
        assert!(corral1.i_condition);
        assert!(corral1.p_condition);
        assert_eq!(corral1.pushes, pushes);

        let corral2 = compute_corral_helper(&game, 5, 4);
        assert!(!corral2.i_condition);
        assert!(corral2.p_condition);
    }

    #[test]
    fn test_pi_corral_7() {
        let game = parse_game(
            r#"
        ########
        #      #
        # $#$ ##
        # $  @#
        ##$ $$#
######### $ # ###
#....  ## $  $  #
##...    $   $  #
#....  ##########
########
"#,
        );

        let mut corral2_pushes = Moves::new();
        corral2_pushes.add(Index(8), Direction::Right);
        corral2_pushes.add(Index(10), Direction::Right);

        let mut corral3_pushes = Moves::new();
        corral3_pushes.add(Index(9), Direction::Left);

        let corral1 = compute_corral_helper(&game, 13, 5);
        assert!(!corral1.i_condition);
        assert!(!corral1.p_condition);

        let corral2 = compute_corral_helper(&game, 14, 7);
        assert!(corral2.i_condition);
        assert!(corral2.p_condition);
        assert_eq!(corral2.pushes, corral2_pushes);

        let corral3 = compute_corral_helper(&game, 8, 7);
        assert!(corral3.i_condition);
        assert!(corral3.p_condition);
        assert_eq!(corral3.pushes, corral3_pushes);
    }

    #[test]
    fn test_pi_corral_8() {
        let mut game = parse_game(
            r#"
######
#.   #
#.$@ #
#.  $#
#  $ #
######
"#,
        );

        let reachable = game.compute_pushes();
        let mut searcher = CorralSearcher::new(Rc::new(Zobrist::new()), 10000);
        let result = searcher.search(&mut game, &reachable);
        assert_eq!(result, CorralResult::Deadlocked);
    }

    #[test]
    fn test_deadlock_1() {
        let mut game = parse_game(
            r#"
#######
#. $  #
#+$   #
#######
"#,
        );

        check_corral_deadlock(&mut game, Direction::Right, DeadlockResult::Ok);
    }

    #[test]
    fn test_deadlock_2() {
        let mut game = parse_game(
            r#"
#######
#. $  #
#.@$  #
#######
"#,
        );

        check_corral_deadlock(&mut game, Direction::Right, DeadlockResult::Deadlocked);
    }

    #[test]
    fn test_deadlock_3() {
        let mut game = parse_game(
            r#"
########
#.   ###
#    ###
#$ @ ###
# #$$  #
#   ## #
# ..   #
########
"#,
        );

        check_corral_deadlock(&mut game, Direction::Down, DeadlockResult::Ok);
    }

    #[test]
    fn test_deadlock_4() {
        let mut game = parse_game(
            r#"
########
#.   ###
#    ###
#$   ###
# #@$  #
#   ## #
# .*   #
########
"#,
        );

        check_corral_deadlock(&mut game, Direction::Right, DeadlockResult::Deadlocked);
    }

    #[test]
    fn test_deadlock_5() {
        let mut game = parse_game(
            r#"
 #########
##   #   ##
#    #    #
#    #    #
#  $*.+   #
####. *####
#   .**   #
#  $$# $  #
#    #    #
##   #   ##
 #########
"#,
        );

        check_corral_deadlock(&mut game, Direction::Down, DeadlockResult::Deadlocked);
    }

    #[test]
    fn test_deadlock_6() {
        let mut game = parse_game(
            r#"
               #####
               #   #
#######  ####### # #
#     #  #  #      #
#     ####$ #   $ ####
#  #    ....## ####  #
#    #####$## $      #
######   #@          #
         #  ##########
         ####
"#,
        );

        check_corral_deadlock(&mut game, Direction::Up, DeadlockResult::Deadlocked);
    }

    fn parse_game(text: &str) -> Game {
        Game::from_text(text.trim_matches('\n')).unwrap()
    }

    fn compute_corral_helper(game: &Game, x: u8, y: u8) -> Corral {
        compute_corral(game, Position(x, y), &game.compute_pushes()).unwrap()
    }

    fn check_corral_deadlock(
        game: &mut Game,
        direction: Direction,
        expected_result: DeadlockResult,
    ) {
        let reachable = game.compute_pushes();
        let box_pos = game.move_position(game.player(), direction).unwrap();
        let corral_pos = game.move_position(box_pos, direction).unwrap();
        let corral = compute_corral(game, corral_pos, &reachable).unwrap();
        let zobrist = Rc::new(Zobrist::new());
        let mut searcher = DeadlockSearcher::new(zobrist, 100);
        let result = searcher.search(game, &corral);
        assert_eq!(result, expected_result);
    }
}
