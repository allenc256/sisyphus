use std::mem::MaybeUninit;

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
