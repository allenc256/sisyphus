use crate::game::{Game, MAX_SIZE, PlayerPos, Position};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Zobrist hash for game states
pub struct Zobrist {
    box_hashes: [[u64; MAX_SIZE]; MAX_SIZE],
    player_hashes: [[u64; MAX_SIZE]; MAX_SIZE],
    player_unknown_hash: u64,
}

impl Zobrist {
    pub fn new() -> Self {
        // Use a seeded PRNG for reproducible Zobrist hashes
        let mut rng = ChaCha8Rng::seed_from_u64(0x123456789abcdef0);

        let mut box_hashes = [[0u64; MAX_SIZE]; MAX_SIZE];
        for row in box_hashes.iter_mut() {
            for cell in row.iter_mut() {
                *cell = rng.next_u64();
            }
        }

        let mut player_hashes = [[0u64; MAX_SIZE]; MAX_SIZE];
        for row in player_hashes.iter_mut() {
            for cell in row.iter_mut() {
                *cell = rng.next_u64();
            }
        }

        let player_unknown_hash = rng.next_u64();

        Zobrist {
            box_hashes,
            player_hashes,
            player_unknown_hash,
        }
    }

    /// Get hash value for a box at a specific position
    pub fn box_hash(&self, x: u8, y: u8) -> u64 {
        self.box_hashes[y as usize][x as usize]
    }

    /// Get hash value for player position
    pub fn player_hash(&self, pos: PlayerPos) -> u64 {
        match pos {
            PlayerPos::Known(Position(x, y)) => self.player_hashes[y as usize][x as usize],
            PlayerPos::Unknown => self.player_unknown_hash,
        }
    }

    /// Compute hash for all boxes in a game state
    pub fn compute_boxes_hash(&self, game: &Game) -> u64 {
        let mut boxes_hash = 0u64;
        for &Position(x, y) in game.box_positions() {
            boxes_hash ^= self.box_hash(x, y);
        }
        boxes_hash
    }

    /// Compute the hash for a game state (boxes hash XOR canonical player position hash)
    pub fn compute_hash(&self, game: &Game) -> u64 {
        let boxes_hash = self.compute_boxes_hash(game);
        let canonical_pos = game.canonical_player_pos();
        boxes_hash ^ self.player_hash(canonical_pos)
    }
}
