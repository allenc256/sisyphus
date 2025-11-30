use arrayvec::ArrayVec;

use crate::{
    bits::{Bitvector, LazyBitboard, Position},
    game::{ALL_DIRECTIONS, Game, MAX_SIZE, Move, Moves, Push, ReachableSet, Tile},
};

struct Corral {
    /// The set of boxes comprising the edge of the corral.
    edge: Bitvector,
    /// The extent of the corral. This includes all boxes within the corral,
    /// including its edge.
    extent: LazyBitboard,
}

pub fn find_pi_corral(game: &Game, reachable: &ReachableSet<Push>) -> Option<Moves<Push>> {
    let mut visited = LazyBitboard::new();
    let mut result = None;
    let mut min_cost = usize::MAX;

    for push in reachable.moves.iter() {
        let box_pos = game.box_position(push.box_index());
        let new_pos = game.move_position(box_pos, push.direction()).unwrap();

        // Look for a corral by examining the other side of a push.
        // Note: we ignore corrals that are not on the other side of a valid
        // player push, since these cannot possibly fulfill the PI conditions
        // necessary for pruning.
        if !reachable.squares.get(new_pos) && !visited.get(new_pos) {
            if let Some(corral) = find_corral(game, new_pos, reachable) {
                visited.set_all(&corral.extent);

                // We found a corral: now check the PI conditions to see if
                // eligible for pruning
                if let Some(pushes) = check_pi_corral_conditions(game, reachable, &corral) {
                    // Keep only the "min cost" corral
                    let cost = pushes.len();
                    if cost < min_cost {
                        result = Some(pushes);
                        min_cost = cost;
                    }
                }
            }
        }
    }

    result
}

fn find_corral(game: &Game, pos: Position, reachable: &ReachableSet<Push>) -> Option<Corral> {
    assert!(!reachable.squares.get(pos));

    let mut stack: ArrayVec<Position, { MAX_SIZE * MAX_SIZE }> = ArrayVec::new();
    let mut extent = LazyBitboard::new();
    let mut edge = Bitvector::new();
    let mut requires_push = false;

    // Start DFS from the given position
    stack.push(pos);
    extent.set(pos);

    // Perform DFS to find full extent of corral
    while let Some(curr_pos) = stack.pop() {
        let is_goal = game.get_tile(curr_pos) == Tile::Goal;

        // We've hit a box
        if let Some(box_idx) = game.box_index(curr_pos) {
            // Box not on goal: corral requires pushes to solve the puzzle
            if !is_goal {
                requires_push = true;
            }
            // If we've hit the edge of the corral, stop exploring further
            if reachable.boxes.contains(box_idx) {
                edge.add(box_idx);
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

    if requires_push {
        Some(Corral { edge, extent })
    } else {
        None
    }
}

fn check_pi_corral_conditions(
    game: &Game,
    reachable: &ReachableSet<Push>,
    corral: &Corral,
) -> Option<Moves<Push>> {
    let mut pushes = Moves::new();

    // Check the PI conditions over the edge boxes
    for box_idx in corral.edge.iter() {
        let box_pos = game.box_position(box_idx);
        for &dir in &ALL_DIRECTIONS {
            if let (Some(next_pos), Some(player_pos)) = (
                game.move_position(box_pos, dir),
                game.move_position(box_pos, dir.reverse()),
            ) {
                // Ignore pushes originating from within the corral
                if corral.extent.get(player_pos) {
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
                if !corral.extent.get(next_pos) {
                    return None;
                }
                // Check P condition: the player must be capable of making the push
                if !reachable.squares.get(player_pos) {
                    return None;
                }
                // Everything checks out for this push
                pushes.add(box_idx, dir);
            }
        }
    }

    Some(pushes)
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

        check_pi_corral(&game, 3, 2, None);
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

        let mut expected_moves = Moves::new();
        expected_moves.add(Index(0), Direction::Left);
        expected_moves.add(Index(1), Direction::Left);

        check_pi_corral(&game, 3, 2, Some(expected_moves));
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

        let mut expected_moves = Moves::new();
        expected_moves.add(Index(1), Direction::Left);
        expected_moves.add(Index(2), Direction::Left);
        expected_moves.add(Index(4), Direction::Left);

        check_pi_corral(&game, 3, 2, Some(expected_moves));
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

        check_pi_corral(&game, 2, 2, None);
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

        check_pi_corral(&game, 2, 2, Some(expected_moves));
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

        let mut expected_moves = Moves::new();
        expected_moves.add(Index(0), Direction::Left);

        check_pi_corral(&game, 3, 2, Some(expected_moves));
        check_pi_corral(&game, 5, 4, None);
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

        let mut corral1_moves = Moves::new();
        corral1_moves.add(Index(8), Direction::Right);
        corral1_moves.add(Index(10), Direction::Right);

        let mut corral2_moves = Moves::new();
        corral2_moves.add(Index(9), Direction::Left);

        check_pi_corral(&game, 13, 5, None);
        check_pi_corral(&game, 14, 7, Some(corral1_moves));
        check_pi_corral(&game, 8, 7, Some(corral2_moves));
    }

    #[test]
    fn test_pi_corral_8() {
        let game = parse_game(
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
        let actual = find_pi_corral(&game, &reachable).unwrap();
        let expected = Moves::new();
        assert_eq!(expected, actual);
    }

    fn parse_game(text: &str) -> Game {
        Game::from_text(text.trim_matches('\n')).unwrap()
    }

    fn check_pi_corral(game: &Game, x: u8, y: u8, expected_result: Option<Moves<Push>>) {
        let reachable = game.compute_pushes();
        let pos = Position(x, y);
        let corral = find_corral(game, pos, &reachable).unwrap();
        let result = check_pi_corral_conditions(game, &reachable, &corral);
        assert_eq!(result, expected_result);
    }
}
