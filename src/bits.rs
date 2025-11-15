use std::mem::MaybeUninit;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Bitvector {
    bits: u64,
}

impl Bitvector {
    pub fn new() -> Self {
        Self { bits: 0 }
    }

    pub fn contains(&self, index: u8) -> bool {
        assert!(index < 64, "index out of bounds");
        (self.bits & (1u64 << index)) != 0
    }

    pub fn add(&mut self, index: u8) {
        assert!(index < 64, "index out of bounds");
        self.bits |= 1u64 << index;
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
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bits == 0 {
            None
        } else {
            let index = self.bits.trailing_zeros() as u8;
            self.bits &= self.bits - 1; // Clear the lowest set bit
            Some(index)
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

    pub fn get(&self, x: u8, y: u8) -> bool {
        assert!(x < 64 && y < 64, "position out of bounds");
        let y = y as usize;
        if (self.initialized & (1u64 << y)) == 0 {
            false
        } else {
            unsafe { (*self.data[y].as_ptr() & (1u64 << x)) != 0 }
        }
    }

    pub fn set(&mut self, x: u8, y: u8) {
        assert!(x < 64 && y < 64, "position out of bounds");
        let y = y as usize;
        if (self.initialized & (1u64 << y)) == 0 {
            self.data[y].write(0);
            self.initialized |= 1u64 << y;
        }
        unsafe {
            *self.data[y].as_mut_ptr() |= 1u64 << x;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitvector_get_set() {
        let mut bv = Bitvector::new();
        assert!(!bv.contains(0));
        assert!(!bv.contains(5));
        assert!(!bv.contains(63));

        bv.add(5);
        assert!(!bv.contains(0));
        assert!(bv.contains(5));
        assert!(!bv.contains(63));

        bv.add(0);
        bv.add(63);
        assert!(bv.contains(0));
        assert!(bv.contains(5));
        assert!(bv.contains(63));
    }

    #[test]
    fn test_bitvector_is_empty() {
        let mut bv = Bitvector::new();
        assert!(bv.is_empty());

        bv.add(0);
        assert!(!bv.is_empty());

        bv.add(63);
        assert!(!bv.is_empty());
    }

    #[test]
    fn test_bitvector_len() {
        let mut bv = Bitvector::new();
        assert_eq!(bv.len(), 0);

        bv.add(0);
        assert_eq!(bv.len(), 1);

        bv.add(5);
        assert_eq!(bv.len(), 2);

        bv.add(63);
        assert_eq!(bv.len(), 3);

        // Setting the same bit again should not change length
        bv.add(5);
        assert_eq!(bv.len(), 3);
    }

    #[test]
    fn test_bitvector_iter() {
        let mut bv = Bitvector::new();
        bv.add(0);
        bv.add(5);
        bv.add(10);
        bv.add(63);

        let indexes: Vec<u8> = bv.iter().collect();
        assert_eq!(indexes, vec![0, 5, 10, 63]);
    }

    #[test]
    fn test_bitvector_iter_empty() {
        let bv = Bitvector::new();
        let indexes: Vec<u8> = bv.iter().collect();
        assert_eq!(indexes, Vec::<u8>::new());
    }

    #[test]
    fn test_bitvector_iter_all() {
        let mut bv = Bitvector::new();
        for i in 0..64 {
            bv.add(i);
        }
        let indexes: Vec<u8> = bv.iter().collect();
        assert_eq!(indexes.len(), 64);
        assert_eq!(indexes, (0..64).collect::<Vec<u8>>());
    }
}
