//! Garbage queue mechanics.

/// The garbage queue, which is a FIFO of incoming garbage lines.
/// The last element is the next group of lines to be added.
/// We can safely assume that no singular attack will send more than [`u32::MAX`] at once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GarbageQueue {
    /// The segments of garbage in the queue. Each segment represents a group of lines to be added.
    pub segments: Vec<u32>,
}

impl GarbageQueue {
    /// Create a new, empty garbage queue.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Recieve garbage from the opponent, and add it to the queue.
    /// The `max` parameter is the maximum total lines that can be in the queue at once. Any excess lines are discarded.
    #[inline]
    pub fn recieve(&mut self, mut amount: u32, max: u32) {
        let current_total: u32 = self.segments.iter().sum();
        if current_total >= max {
            return;
        }

        if current_total + amount > max {
            amount = max - current_total;
        }

        if amount > 0 {
            self.segments.push(amount);
        }
    }

    /// Tanks or cancels up to `max` lines of garbage at once, returning the segments tanked.
    ///
    /// The caller is able to determine if this is a tank or cancel by optionally discarding the result.
    #[inline]
    #[must_use]
    pub fn tank(&mut self, max: u32) -> Vec<u32> {
        let mut sent = Vec::new();
        let mut remaining = max;

        while remaining > 0 && !self.segments.is_empty() {
            let segment = self.segments.pop().unwrap();
            if segment <= remaining {
                sent.push(segment);
                remaining -= segment;
            } else {
                self.segments.insert(0, segment - remaining);
                sent.push(remaining);
                remaining = 0;
            }
        }

        sent
    }

    /// Returns the total number of lines in the queue.
    #[inline]
    #[must_use]
    pub fn total(&self) -> u32 {
        self.segments.iter().sum()
    }

    // TODO: implement this.
    /// Returns the cleanliness of the garbage queue, which is a measure of how "clean" the queue is, within `[0.0, 1.0]`
    /// A cleanliness of 1.0 means the queue is completely clean, while a cleanliness of 0.0 means the queue is completely cheese.
    ///
    /// "Cheese" is a queue with many small segments, while "clean" is a queue with large segments.
    /// Typically, a "clean" send is one that is >= 4 lines.
    #[inline]
    #[must_use]
    pub fn cleanliness(&self) -> f64 {
        todo!()
    }
}

impl Default for GarbageQueue {
    fn default() -> Self {
        Self::new()
    }
}
