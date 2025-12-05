use std::collections::VecDeque;

const NUM_BUCKETS: usize = 4096;
const NUM_WORDS: usize = NUM_BUCKETS / 64;

/// A bucketed priority queue implementation which supports O(1) pop-min.
/// Priority values must lie within the range 0..4096
pub struct PriorityQueue<T> {
    buckets: [VecDeque<T>; NUM_BUCKETS],
    bitmap: [u64; NUM_WORDS],
    summary: u64,
}

impl<T> PriorityQueue<T> {
    pub fn new() -> Self {
        Self {
            buckets: std::array::from_fn(|_| VecDeque::new()),
            bitmap: [0; NUM_WORDS],
            summary: 0,
        }
    }

    pub fn push(&mut self, priority: usize, item: T) {
        assert!(priority < NUM_BUCKETS, "priority must be < {}", NUM_BUCKETS);
        self.buckets[priority].push_back(item);

        // Update bitmap
        let word_idx = priority / 64;
        let bit_idx = priority % 64;
        self.bitmap[word_idx] |= 1u64 << bit_idx;
        self.summary |= 1u64 << word_idx;
    }

    pub fn pop_min(&mut self) -> Option<T> {
        // Find first non-empty word in summary
        if self.summary == 0 {
            return None;
        }
        let word_idx = self.summary.trailing_zeros() as usize;

        // Find first non-empty bucket in that word
        let bit_idx = self.bitmap[word_idx].trailing_zeros() as usize;
        let priority = word_idx * 64 + bit_idx;

        // Pop item from bucket
        let item = self.buckets[priority].pop_front()?;

        // Update bitmap if bucket is now empty
        if self.buckets[priority].is_empty() {
            self.bitmap[word_idx] &= !(1u64 << bit_idx);
            // Update summary if word is now empty
            if self.bitmap[word_idx] == 0 {
                self.summary &= !(1u64 << word_idx);
            }
        }

        Some(item)
    }
}

impl<T> Default for PriorityQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop_single() {
        let mut pq = PriorityQueue::new();
        pq.push(10, "hello");
        assert_eq!(pq.pop_min(), Some("hello"));
        assert_eq!(pq.pop_min(), None);
    }

    #[test]
    fn test_push_pop_ordered() {
        let mut pq = PriorityQueue::new();
        pq.push(10, "low");
        pq.push(5, "lower");
        pq.push(15, "high");

        assert_eq!(pq.pop_min(), Some("lower"));
        assert_eq!(pq.pop_min(), Some("low"));
        assert_eq!(pq.pop_min(), Some("high"));
        assert_eq!(pq.pop_min(), None);
    }

    #[test]
    fn test_push_pop_same_priority() {
        let mut pq = PriorityQueue::new();
        pq.push(10, "first");
        pq.push(10, "second");
        pq.push(10, "third");

        assert_eq!(pq.pop_min(), Some("first"));
        assert_eq!(pq.pop_min(), Some("second"));
        assert_eq!(pq.pop_min(), Some("third"));
        assert_eq!(pq.pop_min(), None);
    }

    #[test]
    fn test_push_pop_mixed() {
        let mut pq = PriorityQueue::new();
        pq.push(100, "a");
        pq.push(50, "b");
        assert_eq!(pq.pop_min(), Some("b"));
        pq.push(25, "c");
        pq.push(75, "d");
        assert_eq!(pq.pop_min(), Some("c"));
        assert_eq!(pq.pop_min(), Some("d"));
        assert_eq!(pq.pop_min(), Some("a"));
    }

    #[test]
    fn test_boundary_priorities() {
        let mut pq = PriorityQueue::new();
        pq.push(0, "min");
        pq.push(NUM_BUCKETS - 1, "max");
        pq.push(2000, "mid");

        assert_eq!(pq.pop_min(), Some("min"));
        assert_eq!(pq.pop_min(), Some("mid"));
        assert_eq!(pq.pop_min(), Some("max"));
    }

    #[test]
    #[should_panic(expected = "priority must be <")]
    fn test_priority_too_large() {
        let mut pq = PriorityQueue::new();
        pq.push(NUM_BUCKETS, "invalid");
    }

    #[test]
    fn test_empty_queue() {
        let mut pq: PriorityQueue<i32> = PriorityQueue::new();
        assert_eq!(pq.pop_min(), None);
    }

    #[test]
    fn test_bitmap_word_boundaries() {
        let mut pq = PriorityQueue::new();
        // Test across word boundaries (each word is 64 buckets)
        pq.push(63, "word0_last");
        pq.push(64, "word1_first");
        pq.push(128, "word2_first");
        pq.push(0, "word0_first");

        assert_eq!(pq.pop_min(), Some("word0_first"));
        assert_eq!(pq.pop_min(), Some("word0_last"));
        assert_eq!(pq.pop_min(), Some("word1_first"));
        assert_eq!(pq.pop_min(), Some("word2_first"));
    }
}
