use crate::distribution::Distribution;
pub struct Rng {
    seed: i32,
}

impl Rng {
    pub fn new_unseeded() -> Self {
        let seed = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() % 2147483647) as i32;
        Self::new(seed)
    }
    pub fn new(seed: i32) -> Self {
        let mut t = seed % 2147483647;
        if t <= 0 {
            t += 2147483646;
        }

        Self { seed: t }
    }

    pub fn next_float(&mut self) -> f64 {
        let n = self.next();
        return (n - 1) as f64 / 2147483646.0;
    }

    pub fn shuffle_array<T>(&mut self, array: &mut [T]) {
        if array.len() == 0 {
            return;
        }

        let mut i = array.len() - 1;
        while i != 0 {
            let r = (self.next_float() * (i as f64 + 1.0)).floor() as usize;
            array.swap(i, r);

            i -= 1;
        }
    }

    pub fn fill_bytes(&mut self, bytes: &mut [u8]) {
        for i in bytes {
            *i = self.next() as u8;
        }
    }

    pub fn sample<D>(&mut self, d: D) -> D::Item
    where
        D: Distribution,
    {
        d.sample(self)
    }

    pub fn next(&mut self) -> i32 {
        self.seed = 16807i32.wrapping_mul(self.seed) % 2147483647;
        self.seed
    }
}
