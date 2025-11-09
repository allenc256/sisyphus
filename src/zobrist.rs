use crate::game::MAX_SIZE;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Zobrist hash for game states
pub struct Zobrist {
    box_hashes: [[u64; MAX_SIZE]; MAX_SIZE],
    player_hashes: [[u64; MAX_SIZE]; MAX_SIZE],
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

        Zobrist {
            box_hashes,
            player_hashes,
        }
    }

    /// Get hash value for a box at a specific position
    pub fn box_hash(&self, x: u8, y: u8) -> u64 {
        self.box_hashes[y as usize][x as usize]
    }

    /// Get hash value for player at a specific position
    pub fn player_hash(&self, x: u8, y: u8) -> u64 {
        self.player_hashes[y as usize][x as usize]
    }
}

