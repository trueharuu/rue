#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coordinates {
    pub x: i8,
    pub y: i8,
}

impl Coordinates {
    pub const fn new(x: i32, y: i32) -> Self {
        Self {
            x: x as i8,
            y: y as i8,
        }
    }

    pub const fn add(self, c: Coordinates) -> Coordinates {
        Coordinates {
            x: self.x.wrapping_add(c.x),
            y: self.y.wrapping_add(c.y),
        }
    }

    pub const fn sub(self, c: Coordinates) -> Coordinates {
        Coordinates {
            x: self.x.wrapping_sub(c.x),
            y: self.y.wrapping_sub(c.y),
        }
    }

    pub const fn pair(self) -> (i8, i8) {
        (self.x, self.y)
    }
}

impl std::ops::Add for Coordinates {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::Sub for Coordinates {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::AddAssign for Coordinates {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl std::ops::SubAssign for Coordinates {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}