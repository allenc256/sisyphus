use crate::bitboard::LazyBitboard;
use crate::game::{Direction, Game, Tile};

pub struct Deadlocks {}

impl Deadlocks {
    pub fn is_freeze_deadlock(game: &Game, x: u8, y: u8) -> bool {
        assert!(game.box_at(x, y).is_some());
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

    fn is_deadlocked(&mut self, game: &Game, x: u8, y: u8) -> bool {
        self.is_frozen(game, x, y) && self.deadlocked
    }

    fn is_frozen(&mut self, game: &Game, x: u8, y: u8) -> bool {
        self.visited.set(x, y);
        let frozen = (self.is_frozen_dir(game, x, y, Direction::Left)
            || self.is_frozen_dir(game, x, y, Direction::Right))
            && (self.is_frozen_dir(game, x, y, Direction::Up)
                || self.is_frozen_dir(game, x, y, Direction::Down));
        if frozen && game.get_tile(x, y) != Tile::Goal {
            self.deadlocked = true;
        }
        frozen
    }

    fn is_frozen_dir(&mut self, game: &Game, x: u8, y: u8, dir: Direction) -> bool {
        if let Some((nx, ny)) = game.push_pos(x, y, dir) {
            if game.box_at(nx, ny).is_some() {
                if self.visited.get(nx, ny) {
                    // In this case, we treat it as if this location had a wall.
                    true
                } else {
                    // Otherwise, we are frozen iff the next box is frozen.
                    self.is_frozen(game, nx, ny)
                }
            } else {
                game.get_tile(nx, ny) == Tile::Wall
            }
        } else {
            true
        }
    }
}
