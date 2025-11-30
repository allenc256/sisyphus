use arrayvec::ArrayVec;

use crate::{
    bits::{Bitvector, LazyBitboard, Position},
    game::{ALL_DIRECTIONS, Game, MAX_SIZE, Move, Moves, Push, ReachableSet, Tile},
};

pub fn find_pi_corral(game: &Game, reachable: &ReachableSet<Push>) -> Option<Moves<Push>> {
    let mut visited = LazyBitboard::new();
    let mut result = None;
    let mut min_cost = usize::MAX;

    for push in reachable.moves.iter() {
        let box_pos = game.box_position(push.box_index());
        let new_pos = game.move_position(box_pos, push.direction()).unwrap();

        // Examine the other side of the push for a PI-corral. Note that we only
        // need to consider corrals that are the other side of a valid player
        // push (any corral NOT on the other side of a player push full the "P"
        // condition of a PI-corral).
        if !reachable.squares.get(new_pos) && !visited.get(new_pos) {
            if let Some((new_pushes, new_cost)) =
                find_pi_corral_helper(game, new_pos, reachable, &mut visited)
            {
                // If we've found a PI-corral, check if this is is the
                // lowest "cost" PI-corral we've found so far. If it is, set
                // the player pushes to this PI-corral's pushes.
                if new_cost < min_cost {
                    result = Some(new_pushes);
                    min_cost = new_cost;
                }
            }
        }
    }

    result
}

fn find_pi_corral_helper(
    game: &Game,
    pos: Position,
    reachable: &ReachableSet<Push>,
    visited: &mut LazyBitboard,
) -> Option<(Moves<Push>, usize)> {
    assert!(!reachable.squares.get(pos));

    let mut stack: ArrayVec<Position, { MAX_SIZE * MAX_SIZE }> = ArrayVec::new();
    let mut locally_visited = LazyBitboard::new();
    let mut edge = Bitvector::new();
    let mut pushes = Moves::new();
    let mut must_be_pushed = false;

    // Start DFS from the given position
    stack.push(pos);
    locally_visited.set(pos);
    visited.set(pos);

    // Perform DFS to find full extent of corral
    while let Some(curr_pos) = stack.pop() {
        let is_goal = game.get_tile(curr_pos) == Tile::Goal;

        // We've hit a box
        if let Some(box_idx) = game.box_index(curr_pos) {
            // Box not on goal: corral requires pushes to solve the puzzle
            if !is_goal {
                must_be_pushed = true;
            }
            // If we've hit the edge of the corral, stop exploring further
            if reachable.boxes.contains(box_idx) {
                edge.add(box_idx);
                continue;
            }
        } else if is_goal {
            // Goal without a box: corral requires pushes to solve the puzzle
            must_be_pushed = true;
        }

        // Otherwise, continue searching in all directions
        for &dir in &ALL_DIRECTIONS {
            if let Some(next_pos) = game.move_position(curr_pos, dir) {
                if game.get_tile(next_pos) != Tile::Wall && !locally_visited.get(next_pos) {
                    stack.push(next_pos);
                    locally_visited.set(next_pos);
                    visited.set(next_pos);
                }
            }
        }
    }

    if !must_be_pushed {
        return None;
    }

    // Check the PI conditions over the edge boxes
    for box_idx in edge.iter() {
        let box_pos = game.box_position(box_idx);
        for &dir in &ALL_DIRECTIONS {
            if let (Some(next_pos), Some(player_pos)) = (
                game.move_position(box_pos, dir),
                game.move_position(box_pos, dir.reverse()),
            ) {
                // Ignore pushes originating from within the corral
                if locally_visited.get(player_pos) {
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
                if !locally_visited.get(next_pos) {
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

    let cost = pushes.len();
    Some((pushes, cost))
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
        let expected_size = 2;

        check_pi_corral(&game, 3, 2, Some((expected_moves, expected_size)));
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
        let expected_size = 3;

        check_pi_corral(&game, 3, 2, Some((expected_moves, expected_size)));
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
        let expected_size = 2;

        check_pi_corral(&game, 2, 2, Some((expected_moves, expected_size)));
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
        let expected_size = 1;

        check_pi_corral(&game, 3, 2, Some((expected_moves, expected_size)));
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
        let corral1_size = 2;

        let mut corral2_moves = Moves::new();
        corral2_moves.add(Index(9), Direction::Left);
        let corral2_size = 1;

        check_pi_corral(&game, 13, 5, None);
        check_pi_corral(&game, 14, 7, Some((corral1_moves, corral1_size)));
        check_pi_corral(&game, 8, 7, Some((corral2_moves, corral2_size)));
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

    fn check_pi_corral(game: &Game, x: u8, y: u8, expected_result: Option<(Moves<Push>, usize)>) {
        let mut visited = LazyBitboard::new();
        let reachable = game.compute_pushes();
        let result = find_pi_corral_helper(game, Position(x, y), &reachable, &mut visited);
        assert_eq!(result, expected_result);
    }
}
