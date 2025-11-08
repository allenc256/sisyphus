use crate::game::MAX_SIZE;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;

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

/// Transposition table for storing visited states
pub struct TranspositionTable {
    visited: HashMap<u64, usize>, // Maps hash to depth at which it was first visited
}

impl TranspositionTable {
    pub fn new() -> Self {
        TranspositionTable {
            visited: HashMap::new(),
        }
    }

    /// Check if a state has been visited at an equal or lesser depth
    /// Returns true if we should skip this state
    pub fn should_skip(&self, hash: u64, depth: usize) -> bool {
        if let Some(&prev_depth) = self.visited.get(&hash) {
            // Skip if we've seen this state at a shallower or equal depth
            depth >= prev_depth
        } else {
            false
        }
    }

    /// Mark a state as visited at the given depth
    pub fn insert(&mut self, hash: u64, depth: usize) {
        self.visited.insert(hash, depth);
    }

    /// Clear the transposition table
    pub fn clear(&mut self) {
        self.visited.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transposition_table_insert_and_check() {
        let mut tt = TranspositionTable::new();
        let hash = 0x123456789abcdef0u64;

        // First visit at depth 5
        assert!(!tt.should_skip(hash, 5));
        tt.insert(hash, 5);

        // Should skip at same or greater depth
        assert!(tt.should_skip(hash, 5));
        assert!(tt.should_skip(hash, 6));

        // Should not skip at lesser depth
        assert!(!tt.should_skip(hash, 4));
    }
}
