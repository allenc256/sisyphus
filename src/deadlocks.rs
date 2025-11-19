use crate::bits::LazyBitboard;
use crate::game::{Direction, Game, GameType, Tile};

pub struct Deadlocks {}

impl Deadlocks {
    // TODO: specialize this to Forward vs Reverse game types
    pub fn is_freeze_deadlock<T: GameType>(game: &Game<T>, x: u8, y: u8) -> bool {
        Frozen::new().is_deadlocked(game, x, y)
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

    fn is_deadlocked<T: GameType>(&mut self, game: &Game<T>, x: u8, y: u8) -> bool {
        assert!(game.box_at(x, y).is_some());
        self.is_frozen(game, x, y) && self.deadlocked
    }

    fn is_frozen<T: GameType>(&mut self, game: &Game<T>, x: u8, y: u8) -> bool {
        if game.get_tile(x, y) == Tile::Wall {
            return true;
        }
        if game.box_at(x, y).is_none() {
            return false;
        }
        if self.visited.get(x, y) {
            return true;
        }
        self.visited.set(x, y);
        let is_frozen_box = (self.is_frozen_dir(game, x, y, Direction::Left)
            || self.is_frozen_dir(game, x, y, Direction::Right))
            && (self.is_frozen_dir(game, x, y, Direction::Up)
                || self.is_frozen_dir(game, x, y, Direction::Down));
        if is_frozen_box && game.get_tile(x, y) != Tile::Goal {
            self.deadlocked = true;
        }
        is_frozen_box
    }

    fn is_frozen_dir<T: GameType>(&mut self, game: &Game<T>, x: u8, y: u8, dir: Direction) -> bool {
        if let Some((nx, ny)) = game.move_pos(x, y, dir) {
            self.is_frozen(game, nx, ny)
        } else {
            true
        }
    }
}
