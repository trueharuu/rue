use std::mem::MaybeUninit;
use std::ops::Index;

pub struct Buffer<T, const N: usize> {
    data: [MaybeUninit<T>; N],
    len: usize,
}

impl<T, const N: usize> Buffer<T, N> {
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }

    pub fn push(&mut self, value: T) {
        assert!(self.len < N, "Buffer overflow");
        self.data[self.len] = MaybeUninit::new(value);
        self.len += 1;
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            Some(unsafe { &*self.data[index].as_ptr() })
        } else {
            None
        }
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index < self.len {
            let value = unsafe { self.data[index].as_ptr().read() };
            for i in index..self.len - 1 {
                self.data[i] = MaybeUninit::new(unsafe { self.data[i + 1].as_ptr().read() });
            }
            self.len -= 1;
            Some(value)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.len).map(move |i| unsafe { &*self.data[i].as_ptr() })
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len > 0 {
            self.len -= 1;
            Some(unsafe { self.data[self.len].as_ptr().read() })
        } else {
            None
        }
    }

    pub fn insert(&mut self, index: usize, value: T) {
        assert!(self.len < N, "Buffer overflow");
        assert!(index <= self.len, "Index out of bounds");
        for i in (index..self.len).rev() {
            self.data[i + 1] = MaybeUninit::new(unsafe { self.data[i].as_ptr().read() });
        }
        self.data[index] = MaybeUninit::new(value);
        self.len += 1;
    }
}

impl<T, const N: usize> Index<usize> for Buffer<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("Index out of bounds")
    }
}

#[allow(clippy::expl_impl_clone_on_copy)]
impl<T, const N: usize> Clone for Buffer<T, N>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let mut new = Self::new();
        for i in 0..self.len {
            new.push(self.get(i).unwrap().clone());
        }
        new
    }
}

impl<T, const N: usize> Copy for Buffer<T, N> where T: Copy {}

impl<T, const N: usize> Default for Buffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> PartialEq for Buffer<T, N>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        if self.len != other.len {
            return false;
        }
        for i in 0..self.len {
            if self.get(i) != other.get(i) {
                return false;
            }
        }
        true
    }
}

impl<T, const N: usize> Eq for Buffer<T, N> where T: Eq {}

impl<T, const N: usize> std::fmt::Debug for Buffer<T, N>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
