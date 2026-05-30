#[derive(Clone, Copy, Debug)]
pub struct GarbageQueue {
    pub segments: [u8; 64],
}

impl GarbageQueue {
    pub const fn new() -> Self {
        Self { segments: [0; 64] }
    }

    pub const fn total(self) -> usize {
        let mut i = 0;
        let mut c = 0;
        while i < 64 {
            if self.segments[i] == 0 {
                break;
            }
            c += self.segments[i] as usize;
            i += 1;
        }

        c
    }

    /// Removes `amount` garbage from the front of the queue, returning the amount of lines sent, if `length <= amount`.
    pub fn remove(&mut self, mut amount: usize) -> usize {
        loop {
            self.downshift();
            if self.segments[0] == 0 {
                return amount;
            }

            if self.segments[0] as usize >= amount {
                self.segments[0] -= amount as u8;
                return 0;
            }

            amount -= self.segments[0] as usize;
            self.segments[0] = 0;
        }
    }

    pub fn accept(&mut self, amount: usize) {
        if amount == 0 {
            return;
        }
        let mut idx = 0;
        while idx < 64 {
            if self.segments[idx] == 0 {
                break;
            }
            idx += 1;
        }

        self.segments[idx] = amount as u8;
    }

    pub fn accept_many(&mut self, amounts: &[usize]) {
        for &amount in amounts {
            self.accept(amount);
        }
    }

    /// Splits the garbage queue into two segments where the first segment is at most `to` lines, returning the front segment.
    pub fn split(&mut self, mut to: usize) -> Self {
        let mut new_queue = Self::new();
        let mut i = 0;
        while i < 64 && to > 0 {
            if self.segments[i] == 0 {
                break;
            }

            if self.segments[i] as usize > to {
                new_queue.segments[i] = to as u8;
                self.segments[i] -= to as u8;
                break;
            }

            new_queue.segments[i] = self.segments[i];
            to -= self.segments[i] as usize;
            self.segments[i] = 0;

            i += 1;
        }

        new_queue
    }

    pub fn downshift(&mut self) {
        let mut start = 0;
        while start < 64 && self.segments[start] == 0 {
            start += 1;
        }

        if start == 64 {
            return;
        }

        // move all elements from start + i to i until we hit the end of the queue
        let mut i = 0;
        while start + i < 64 {
            self.segments[i] = self.segments[start + i];
            i += 1;
        }
    }
}

impl Default for GarbageQueue {
    fn default() -> Self {
        Self::new()
    }
}
