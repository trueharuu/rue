#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct Reward {
    pub value: i32,
    pub attack: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Default)]
pub struct Value {
    pub value: i32,
    pub spike: i32,
}

impl std::ops::Add for Value {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Value {
            value: self.value + rhs.value,
            spike: self.spike + rhs.spike,
        }
    }
}

impl std::ops::Add<Reward> for Value {
    type Output = Self;
    fn add(self, rhs: Reward) -> Self {
        Value {
            value: self.value + rhs.value,
            spike: if rhs.attack == -1 {
                0
            } else {
                self.spike + rhs.attack
            },
        }
    }
}

impl std::ops::Div<usize> for Value {
    type Output = Self;
    fn div(self, rhs: usize) -> Self {
        Value {
            value: self.value / rhs as i32,
            spike: self.spike / rhs as i32,
        }
    }
}

impl std::ops::Mul<usize> for Value {
    type Output = Self;
    fn mul(self, rhs: usize) -> Self {
        Value {
            value: self.value * rhs as i32,
            spike: self.spike * rhs as i32,
        }
    }
}
