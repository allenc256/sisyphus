use std::{fmt, mem::MaybeUninit};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Index(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position(pub u8, pub u8);

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Bitvector {
    bits: u64,
}

impl Bitvector {
    pub fn new() -> Self {
        Self { bits: 0 }
    }

    pub fn contains(&self, index: Index) -> bool {
        debug_assert!(index.0 < 64, "index out of bounds");
        (self.bits & (1u64 << index.0)) != 0
    }

    pub fn add(&mut self, index: Index) {
        debug_assert!(index.0 < 64, "index out of bounds");
        self.bits |= 1u64 << index.0;
    }

    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }

    pub fn len(&self) -> usize {
        self.bits.count_ones() as usize
    }

    pub fn union(&self, other: &Bitvector) -> Bitvector {
        Bitvector {
            bits: self.bits | other.bits,
        }
    }

    pub fn iter(&self) -> BitvectorIter {
        BitvectorIter { bits: self.bits }
    }
}

pub struct BitvectorIter {
    bits: u64,
}

impl Iterator for BitvectorIter {
    type Item = Index;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bits == 0 {
            None
        } else {
            let index = self.bits.trailing_zeros() as u8;
            self.bits &= self.bits - 1; // Clear the lowest set bit
            Some(Index(index))
        }
    }
}

pub struct LazyBitboard {
    data: [MaybeUninit<u64>; 64],
    initialized: u64,
}

impl LazyBitboard {
    pub fn new() -> Self {
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            initialized: 0,
        }
    }

    pub fn get(&self, pos: Position) -> bool {
        debug_assert!(pos.0 < 64 && pos.1 < 64, "position out of bounds");
        let y = pos.1 as usize;
        if (self.initialized & (1u64 << y)) == 0 {
            false
        } else {
            unsafe { (*self.data[y].as_ptr() & (1u64 << pos.0)) != 0 }
        }
    }

    pub fn set(&mut self, pos: Position) {
        assert!(pos.0 < 64 && pos.1 < 64, "position out of bounds");
        let y = pos.1 as usize;
        if (self.initialized & (1u64 << y)) == 0 {
            self.data[y].write(0);
            self.initialized |= 1u64 << y;
        }
        unsafe {
            *self.data[y].as_mut_ptr() |= 1u64 << pos.0;
        }
    }

    pub fn clear(&mut self) {
        self.initialized = 0;
    }
}

pub struct Bitboard {
    data: [u64; 64],
}

impl Bitboard {
    pub fn new() -> Self {
        Self { data: [0u64; 64] }
    }

    pub fn get(&self, pos: Position) -> bool {
        debug_assert!(pos.0 < 64 && pos.1 < 64, "position out of bounds");
        self.data[pos.1 as usize] & (1u64 << pos.0) != 0
    }

    pub fn set(&mut self, pos: Position) {
        debug_assert!(pos.0 < 64 && pos.1 < 64, "position out of bounds");
        self.data[pos.1 as usize] |= 1u64 << pos.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitvector_get_set() {
        let mut bv = Bitvector::new();
        assert!(!bv.contains(Index(0)));
        assert!(!bv.contains(Index(5)));
        assert!(!bv.contains(Index(63)));

        bv.add(Index(5));
        assert!(!bv.contains(Index(0)));
        assert!(bv.contains(Index(5)));
        assert!(!bv.contains(Index(63)));

        bv.add(Index(0));
        bv.add(Index(63));
        assert!(bv.contains(Index(0)));
        assert!(bv.contains(Index(5)));
        assert!(bv.contains(Index(63)));
    }

    #[test]
    fn test_bitvector_is_empty() {
        let mut bv = Bitvector::new();
        assert!(bv.is_empty());

        bv.add(Index(0));
        assert!(!bv.is_empty());

        bv.add(Index(63));
        assert!(!bv.is_empty());
    }

    #[test]
    fn test_bitvector_len() {
        let mut bv = Bitvector::new();
        assert_eq!(bv.len(), 0);

        bv.add(Index(0));
        assert_eq!(bv.len(), 1);

        bv.add(Index(5));
        assert_eq!(bv.len(), 2);

        bv.add(Index(63));
        assert_eq!(bv.len(), 3);

        // Setting the same bit again should not change length
        bv.add(Index(5));
        assert_eq!(bv.len(), 3);
    }

    #[test]
    fn test_bitvector_iter() {
        let mut bv = Bitvector::new();
        bv.add(Index(0));
        bv.add(Index(5));
        bv.add(Index(10));
        bv.add(Index(63));

        let indexes: Vec<Index> = bv.iter().collect();
        assert_eq!(indexes, vec![Index(0), Index(5), Index(10), Index(63)]);
    }

    #[test]
    fn test_bitvector_iter_empty() {
        let bv = Bitvector::new();
        let indexes: Vec<Index> = bv.iter().collect();
        assert_eq!(indexes, Vec::<Index>::new());
    }

    #[test]
    fn test_bitvector_iter_all() {
        let mut bv = Bitvector::new();
        for i in 0..64 {
            bv.add(Index(i));
        }
        let indexes: Vec<Index> = bv.iter().collect();
        assert_eq!(indexes.len(), 64);
        assert_eq!(indexes, (0..64).map(|i| Index(i as u8)).collect::<Vec<_>>());
    }
}
