use std::mem::MaybeUninit;

use rue_core::placement::Move;

const CAP: usize = 4096;
const MASK: usize = CAP - 1;

pub struct Queue {
    buf: [MaybeUninit<Move>; CAP],
    front: usize,
    back: usize,
}

impl Queue {
    #[inline]
    pub fn new() -> Self {
        Self {
            // SAFETY: An uninitialized `[MaybeUninit<_>; CAP]` is valid.
            buf: unsafe { MaybeUninit::uninit().assume_init() },
            front: 0,
            back: 0,
        }
    }

    #[inline]
    pub fn push_back(&mut self, val: Move) {
        assert!(self.back - self.front != CAP, "queue full");
        self.buf[self.back & MASK].write(val);
        self.back = self.back.wrapping_add(1);
    }

    #[inline]
    pub fn pop_front(&mut self) -> Option<Move> {
        if self.front == self.back {
            return None;
        }
        let val = unsafe { self.buf[self.front & MASK].assume_init_read() };
        self.front = self.front.wrapping_add(1);
        Some(val)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.front == self.back
    }
}
