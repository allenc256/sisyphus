use arrayvec::ArrayVec;

use crate::bits::Bitvector;
use crate::game::{ALL_DIRECTIONS, Direction, Game, Index, MAX_BOXES, Position, Tile};

/// Computes the set of boxes which are currently effectively frozen
pub fn compute_frozen_boxes(game: &Game) -> Bitvector {
    let mut result = Bitvector::new();
    for box_idx in 0..game.box_count() {
        let box_idx = Index(box_idx as u8);
        if !result.contains(box_idx) {
            let frozen = compute_new_frozen_boxes(result, game, box_idx);
            result.add_all(&frozen);
        }
    }
    result
}

/// Incrementally compute boxes which are newly frozen after box_idx has been
/// pushed to its current location.
pub fn compute_new_frozen_boxes(frozen: Bitvector, game: &Game, box_idx: Index) -> Bitvector {
    assert!(!frozen.contains(box_idx));

    // Find all boxes which might become frozen
    let candidates = find_candidates(frozen, game, box_idx);
    // Mark all candidate boxes as frozen initially
    let mut candidates_frozen = candidates;
    // Mark all candidates a needing to be checked
    let mut to_check = candidates;

    while let Some(box_idx) = to_check.pop() {
        let pos = game.box_position(box_idx);
        if check_unfrozen(game, pos, &candidates, &candidates_frozen) {
            candidates_frozen.remove(box_idx);

            // Whenever we unfreeze a box, "wake up" its neighbors to be checked
            // again for unfreezing
            for &dir in &ALL_DIRECTIONS {
                if let Some(next_pos) = game.move_position(pos, dir) {
                    if let Some(next_box_idx) = game.box_index(next_pos) {
                        if candidates_frozen.contains(next_box_idx) {
                            to_check.add(next_box_idx);
                        }
                    }
                }
            }
        }
    }

    candidates_frozen
}

fn find_candidates(frozen: Bitvector, game: &Game, box_idx: Index) -> Bitvector {
    let mut candidates = Bitvector::new();
    let mut stack: ArrayVec<Index, MAX_BOXES> = ArrayVec::new();

    candidates.add(box_idx);
    stack.push(box_idx);

    while let Some(box_idx) = stack.pop() {
        let pos = game.box_position(box_idx);
        for &dir in &ALL_DIRECTIONS {
            if let Some(next_pos) = game.move_position(pos, dir) {
                if let Some(next_box_idx) = game.box_index(next_pos) {
                    if !candidates.contains(next_box_idx) && !frozen.contains(next_box_idx) {
                        candidates.add(next_box_idx);
                        stack.push(next_box_idx);
                    }
                }
            }
        }
    }

    candidates
}

fn check_unfrozen_dir(
    game: &Game,
    pos: Position,
    dir: Direction,
    candidates: &Bitvector,
    candidates_frozen: &Bitvector,
) -> bool {
    if let Some(next_pos) = game.move_position(pos, dir) {
        if let Some(next_box_idx) = game.box_index(next_pos) {
            if candidates.contains(next_box_idx) {
                // Candidate box: check candidates_frozen
                !candidates_frozen.contains(next_box_idx)
            } else {
                // Non-candidate box: must be frozen
                false
            }
        } else {
            // No box: check for a wall
            game.get_tile(next_pos) != Tile::Wall
        }
    } else {
        // Out-of-bounds
        true
    }
}

fn check_dead_square_dir(game: &Game, pos: Position, dir: Direction) -> bool {
    if let Some(next_pos) = game.move_position(pos, dir) {
        game.is_dead_square(next_pos)
    } else {
        true
    }
}

fn check_unfrozen_vertical(
    game: &Game,
    pos: Position,
    candidates: &Bitvector,
    candidates_frozen: &Bitvector,
) -> bool {
    check_unfrozen_dir(game, pos, Direction::Up, candidates, candidates_frozen)
        && check_unfrozen_dir(game, pos, Direction::Down, candidates, candidates_frozen)
        && !(check_dead_square_dir(game, pos, Direction::Up)
            && check_dead_square_dir(game, pos, Direction::Down))
}

fn check_unfrozen_horizontal(
    game: &Game,
    pos: Position,
    candidates: &Bitvector,
    candidates_frozen: &Bitvector,
) -> bool {
    check_unfrozen_dir(game, pos, Direction::Left, candidates, candidates_frozen)
        && check_unfrozen_dir(game, pos, Direction::Right, candidates, candidates_frozen)
        && !(check_dead_square_dir(game, pos, Direction::Left)
            && check_dead_square_dir(game, pos, Direction::Right))
}

fn check_unfrozen(
    game: &Game,
    pos: Position,
    candidates: &Bitvector,
    candidates_frozen: &Bitvector,
) -> bool {
    check_unfrozen_horizontal(game, pos, candidates, candidates_frozen)
        || check_unfrozen_vertical(game, pos, candidates, candidates_frozen)
}
