use arrayvec::ArrayVec;

use crate::bits::{Bitboard, Bitvector, LazyBitboard};
use crate::game::{ALL_DIRECTIONS, Direction, Game, Index, MAX_BOXES, Position, Tile};

pub struct Deadlocks {}

impl Deadlocks {
    pub fn is_freeze_deadlock(game: &Game, pos: Position) -> bool {
        Frozen::new().is_deadlocked(game, pos)
    }
}

struct Frozen {
    visited: LazyBitboard,
    deadlocked: bool,
}

impl Frozen {
    fn new() -> Self {
        Self {
            visited: LazyBitboard::new(),
            deadlocked: false,
        }
    }

    fn is_deadlocked(&mut self, game: &Game, pos: Position) -> bool {
        assert!(game.box_index(pos).is_some());
        self.is_frozen(game, pos) && self.deadlocked
    }

    fn is_frozen(&mut self, game: &Game, pos: Position) -> bool {
        if game.get_tile(pos) == Tile::Wall {
            return true;
        }
        if game.box_index(pos).is_none() {
            return false;
        }
        if self.visited.get(pos) {
            return true;
        }
        self.visited.set(pos);
        let is_frozen_box = (self.is_frozen_dir(game, pos, Direction::Left)
            || self.is_frozen_dir(game, pos, Direction::Right))
            && (self.is_frozen_dir(game, pos, Direction::Up)
                || self.is_frozen_dir(game, pos, Direction::Down));
        if is_frozen_box && game.get_tile(pos) != Tile::Goal {
            self.deadlocked = true;
        }
        is_frozen_box
    }

    fn is_frozen_dir(&mut self, game: &Game, pos: Position, dir: Direction) -> bool {
        if let Some(next_pos) = game.move_position(pos, dir) {
            self.is_frozen(game, next_pos)
        } else {
            true
        }
    }
}

pub struct FrozenSquares {
    frozen: Bitboard,
    deadlock_count: u8,
}

impl FrozenSquares {
    pub fn new(game: &Game) -> Self {
        let mut frozen = Bitboard::new();

        for y in 0..game.height() {
            for x in 0..game.width() {
                let pos = Position(x, y);
                if game.get_tile(pos) == Tile::Wall {
                    frozen.set(pos);
                }
            }
        }

        Self {
            frozen,
            deadlock_count: 0,
        }
    }

    pub fn deadlocked(&self) -> bool {
        self.deadlock_count > 0
    }

    pub fn update_after_push(&mut self, game: &Game, pos: Position) -> Bitvector {
        assert!(!self.frozen.get(pos));
        assert!(game.box_index(pos).is_some());

        let mut boxes = Bitvector::new();
        let mut visited = LazyBitboard::new();
        let mut to_visit: ArrayVec<Position, MAX_BOXES> = ArrayVec::new();
        let mut to_process: ArrayVec<Position, MAX_BOXES> = ArrayVec::new();

        to_visit.push(pos);

        // First pass: Build post-order traversal using to_visit stack
        while let Some(pos) = to_visit.pop() {
            visited.set(pos);
            to_process.push(pos);
            for &dir in &ALL_DIRECTIONS {
                if let Some(next_pos) = game.move_position(pos, dir) {
                    if !visited.get(next_pos)
                        && game.box_index(next_pos).is_some()
                        && !self.frozen.get(next_pos)
                    {
                        to_visit.push(next_pos);
                    }
                }
            }
        }

        // Second pass: Process in reverse order (post-order = children before parents)
        visited.clear();
        while let Some(pos) = to_process.pop() {
            visited.set(pos);
            if self.check_horizontal(game, pos, &visited)
                && self.check_vertical(game, pos, &visited)
            {
                self.frozen.set(pos);
                boxes.add(game.box_index(pos).unwrap());
                if game.get_tile(pos) != Tile::Goal {
                    self.deadlock_count += 1;
                }
            }
        }

        boxes
    }

    fn check_horizontal(&self, game: &Game, pos: Position, visited: &LazyBitboard) -> bool {
        self.check_direction(game, pos, Direction::Left, visited)
            || self.check_direction(game, pos, Direction::Right, visited)
    }

    fn check_vertical(&self, game: &Game, pos: Position, visited: &LazyBitboard) -> bool {
        self.check_direction(game, pos, Direction::Up, visited)
            || self.check_direction(game, pos, Direction::Down, visited)
    }

    fn check_direction(
        &self,
        game: &Game,
        pos: Position,
        dir: Direction,
        visited: &LazyBitboard,
    ) -> bool {
        game.move_position(pos, dir).map_or(true, |next_pos| {
            if game.box_index(next_pos).is_some() {
                if visited.get(next_pos) {
                    self.frozen.get(next_pos)
                } else {
                    true
                }
            } else {
                game.get_tile(next_pos) == Tile::Wall
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::game::Push;

    use super::*;

    #[test]
    fn test_frozen_squares_new() {
        let game = parse_game(
            r#"
#####
#@ *#
#####
"#,
        );
        let frozen = FrozenSquares::new(&game);

        assert!(frozen.frozen.get(Position(0, 0)));
        assert!(frozen.frozen.get(Position(2, 2)));
        assert!(!frozen.frozen.get(Position(1, 1)));
        assert!(!frozen.frozen.get(Position(2, 1)));
        assert!(!frozen.frozen.get(Position(3, 1)));
    }

    #[test]
    fn test_frozen_squares_update_after_push() {
        let game = parse_game(
            r#"
#####
#@ *#
#####
"#,
        );
        let mut frozen = FrozenSquares::new(&game);

        assert!(!frozen.frozen.get(Position(3, 1)));
        frozen.update_after_push(&game, game.box_position(Index(0)));
        assert!(frozen.frozen.get(Position(3, 1)));
    }

    #[test]
    fn test_frozen_squares_recursive_1() {
        let game = parse_game(
            r#"
#####
#   #
#  *#
#@ *#
#   #
#####
"#,
        );
        let mut frozen = FrozenSquares::new(&game);

        for &pos in game.box_positions() {
            assert!(!frozen.frozen.get(pos));
        }
        frozen.update_after_push(&game, Position(3, 2));

        for &pos in game.box_positions() {
            assert!(frozen.frozen.get(pos));
        }
    }

    #[test]
    fn test_frozen_squares_recursive_2() {
        let game = parse_game(
            r#"
########
#      #
#  **  #
#@ *   #
#      #
########
"#,
        );
        let mut frozen = FrozenSquares::new(&game);

        for &pos in game.box_positions() {
            assert!(!frozen.frozen.get(pos));
        }
        frozen.update_after_push(&game, Position(3, 2));

        for &pos in game.box_positions() {
            assert!(!frozen.frozen.get(pos));
        }
        assert_eq!(frozen.deadlock_count, 0);
    }

    #[test]
    fn test_frozen_squares_recursive_3() {
        let game = parse_game(
            r#"
########
#     .#
#  $*  #
#@ *$  #
#     .#
########
"#,
        );
        let mut frozen = FrozenSquares::new(&game);

        for &pos in game.box_positions() {
            assert!(!frozen.frozen.get(pos));
        }
        frozen.update_after_push(&game, Position(3, 2));

        for &pos in game.box_positions() {
            assert!(frozen.frozen.get(pos));
        }
        assert_eq!(frozen.deadlock_count, 2);
    }

    #[test]
    fn test_frozen_squares_recursive_incremental() {
        let mut game = parse_game(
            r#"
########
#      #
#    $ #
#@   $ #
#..    #
########
"#,
        );
        let mut frozen = FrozenSquares::new(&game);

        game.push(Push::new(Index(0), Direction::Right));

        let boxes = frozen.update_after_push(&game, game.box_position(Index(0)));

        for &pos in game.box_positions() {
            assert!(frozen.frozen.get(pos));
        }
        assert_eq!(frozen.deadlock_count, 2);
    }

    fn parse_game(text: &str) -> Game {
        Game::from_text(text.trim_matches('\n')).unwrap()
    }
}
