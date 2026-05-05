use std::ops::{Range, RangeFrom, RangeInclusive, RangeTo, RangeToInclusive};

use crate::rng::Rng;

pub trait Distribution {
    type Item;
    fn sample(self, rng: &mut Rng) -> Self::Item;
}

macro_rules! impl_primitive {
    ($t:ident) => {
        impl Distribution for Range<$t> {
            type Item = $t;
            fn sample(self, rng: &mut Rng) -> Self::Item {
                let start = self.start;
                let end = self.end;
                let width = end - start;

                assert!(width > 0);
                let max = $t::MAX;
                let zone = max - (max % width);

                loop {
                    let x = (rng.next() & i32::MAX) as $t;
                    if x < zone {
                        return start + (x % width);
                    }
                }
            }
        }

        impl Distribution for RangeFrom<$t> {
            type Item = $t;
            fn sample(self, rng: &mut Rng) -> Self::Item {
                let start = self.start;
                let end = $t::MAX;
                let width = end - start;

                assert!(width > 0);
                let max = $t::MAX;
                let zone = max - (max % width);

                loop {
                    let x = (rng.next() & i32::MAX) as $t;
                    if x < zone {
                        return start + (x % width);
                    }
                }
            }
        }

        impl Distribution for RangeTo<$t> {
            type Item = $t;
            fn sample(self, rng: &mut Rng) -> Self::Item {
                let start = $t::MIN;
                let end = self.end;
                let width = end - start;

                assert!(width > 0);
                let max = $t::MAX;
                let zone = max - (max % width);

                loop {
                    let x = (rng.next() & i32::MAX) as $t;
                    if x < zone {
                        return start + (x % width);
                    }
                }
            }
        }

        impl Distribution for RangeInclusive<$t> {
            type Item = $t;
            fn sample(self, rng: &mut Rng) -> Self::Item {
                let start = self.start();
                let end = self.end() + 1;
                let width = end - start;

                assert!(width > 0);
                let max = $t::MAX;
                let zone = max - (max % width);

                loop {
                    let x = (rng.next() & i32::MAX) as $t;
                    if x < zone {
                        return start + (x % width);
                    }
                }
            }
        }

        impl Distribution for RangeToInclusive<$t> {
            type Item = $t;
            fn sample(self, rng: &mut Rng) -> Self::Item {
                let start = $t::MIN;
                let end = self.end + 1;
                let width = end - start;

                assert!(width > 0);
                let max = $t::MAX;
                let zone = max - (max % width);

                loop {
                    let x = (rng.next() & i32::MAX) as $t;
                    if x < zone {
                        return start + (x % width);
                    }
                }
            }
        }
    };
}

impl_primitive!(u8);
impl_primitive!(u16);
impl_primitive!(u32);
impl_primitive!(u64);
impl_primitive!(u128);
impl_primitive!(usize);
impl_primitive!(i8);
impl_primitive!(i16);
impl_primitive!(i32);
impl_primitive!(i64);
impl_primitive!(i128);
impl_primitive!(isize);
