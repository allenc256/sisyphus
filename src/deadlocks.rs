use std::rc::Rc;

use arrayvec::ArrayVec;

use crate::bits::Bitvector;
use crate::game::{ALL_DIRECTIONS, Direction, Game, Index, MAX_BOXES, Tile};
use crate::zobrist::Zobrist;

pub trait FrozenBoxes {
    fn compute_frozen(&mut self, game: &Game, box_idx: Index) -> Option<(Bitvector, u64)>;
    fn clear_frozen(&mut self, boxes: Bitvector);
}

pub struct ReverseFrozenBoxes;

impl FrozenBoxes for ReverseFrozenBoxes {
    fn compute_frozen(&mut self, _game: &Game, _box_idx: Index) -> Option<(Bitvector, u64)> {
        Some((Bitvector::new(), 0))
    }

    fn clear_frozen(&mut self, _boxes: Bitvector) {
        // no-op
    }
}

pub struct ForwardFrozenBoxes {
    frozen: Bitvector,
    zobrist: Rc<Zobrist>,
}

impl ForwardFrozenBoxes {
    pub fn new(game: &Game, zobrist: Rc<Zobrist>) -> Self {
        let mut result = Self {
            frozen: Bitvector::new(),
            zobrist,
        };
        for box_idx in 0..game.box_count() {
            let box_idx = Index(box_idx as u8);
            if !result.frozen.contains(box_idx) {
                let result = result.compute_frozen(game, box_idx);
                assert!(result.is_some());
            }
        }
        result
    }

    fn compute_hash(&self, game: &Game) -> u64 {
        let mut hash = 0u64;
        for box_idx in self.frozen.iter() {
            hash ^= self.zobrist.box_hash(game.box_position(box_idx));
        }
        hash
    }
}

/*

Still wrong!
------------

Frozen state:
 #######
 #     #
 # $   #
 # $$###
 # #$ #
 # #  #
 # #  #
## ## ##
#    *.#######
# ###.**@    #
#   #.####   #
### #.# $    #
  #  .    #  #
  #########  #
          ####

Why aren't the squares in the upper-left considered a deadlock?
I think I should just rewrite the logic to use real recursion instead of the stack-based approach.

 */

impl FrozenBoxes for ForwardFrozenBoxes {
    fn compute_frozen(&mut self, game: &Game, box_idx: Index) -> Option<(Bitvector, u64)> {
        let mut deadlocked = false;
        let mut boxes = Bitvector::new();
        let mut visited = Bitvector::new();
        let mut to_visit: ArrayVec<Index, MAX_BOXES> = ArrayVec::new();
        let mut to_process: ArrayVec<Index, MAX_BOXES> = ArrayVec::new();
        let mut frozen = self.frozen;

        assert!(!self.frozen.contains(box_idx));

        visited.add(box_idx);
        to_visit.push(box_idx);

        // First pass: Build post-order traversal using to_visit stack
        while let Some(box_idx) = to_visit.pop() {
            to_process.push(box_idx);
            boxes.add(box_idx);
            let pos = game.box_position(box_idx);
            for &dir in &ALL_DIRECTIONS {
                if let Some(next_pos) = game.move_position(pos, dir) {
                    if let Some(next_box_idx) = game.box_index(next_pos) {
                        if !visited.contains(next_box_idx) && !self.frozen.contains(next_box_idx) {
                            visited.add(next_box_idx);
                            to_visit.push(next_box_idx);
                        }
                    }
                }
            }
        }

        // Second pass: Process in reverse order (post-order = children before parents)
        visited = Bitvector::new();
        while let Some(box_idx) = to_process.pop() {
            visited.add(box_idx);
            if check_horizontal(game, box_idx, &frozen, &visited)
                && check_vertical(game, box_idx, &frozen, &visited)
            {
                frozen.add(box_idx);
                let pos = game.box_position(box_idx);
                if game.get_tile(pos) != Tile::Goal {
                    deadlocked = true;
                }
            } else {
                return Some((Bitvector::new(), self.compute_hash(game)));
            }
        }

        if deadlocked {
            return None;
        }

        self.frozen = frozen;
        Some((boxes, self.compute_hash(game)))
    }

    fn clear_frozen(&mut self, boxes: Bitvector) {
        assert!(self.frozen.contains_all(&boxes));
        self.frozen.remove_all(&boxes);
    }
}

fn check_horizontal(game: &Game, box_idx: Index, frozen: &Bitvector, visited: &Bitvector) -> bool {
    check_direction(game, box_idx, Direction::Left, frozen, visited)
        || check_direction(game, box_idx, Direction::Right, frozen, visited)
}

fn check_vertical(game: &Game, box_idx: Index, frozen: &Bitvector, visited: &Bitvector) -> bool {
    check_direction(game, box_idx, Direction::Up, frozen, visited)
        || check_direction(game, box_idx, Direction::Down, frozen, visited)
}

fn check_direction(
    game: &Game,
    box_idx: Index,
    dir: Direction,
    frozen: &Bitvector,
    visited: &Bitvector,
) -> bool {
    let pos = game.box_position(box_idx);
    if let Some(next_pos) = game.move_position(pos, dir) {
        if let Some(next_box_idx) = game.box_index(next_pos) {
            // Box: check if it was already verified frozen or not previously visited
            if visited.contains(next_box_idx) {
                frozen.contains(next_box_idx)
            } else {
                true
            }
        } else {
            // No box: check for a wall
            game.get_tile(next_pos) == Tile::Wall
        }
    } else {
        // Out of bounds: treat this like a wall
        true
    }
}
