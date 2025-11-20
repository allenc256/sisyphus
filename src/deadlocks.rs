use crate::bits::LazyBitboard;
use crate::game::{Direction, Game, Position, Tile};

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
