#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mino {
    I,
    O,
    T,
    L,
    J,
    S,
    Z,
}

impl Mino {
    pub const fn bag() -> [Mino; 7] {
        [
            Mino::T,
            Mino::I,
            Mino::J,
            Mino::L,
            Mino::O,
            Mino::S,
            Mino::Z,
        ]
    }

    pub const fn idx(self) -> usize {
        match self {
            Mino::T => 0,
            Mino::I => 1,
            Mino::J => 2,
            Mino::L => 3,
            Mino::O => 4,
            Mino::S => 5,
            Mino::Z => 6,
        }
    }

    pub const fn blocks(&self) -> [(i8, i8); 4] {
        match self {
            Mino::Z => [(-1, 1), (0, 1), (0, 0), (1, 0)],
            Mino::S => [(-1, 0), (0, 0), (0, 1), (1, 1)],
            Mino::I => [(-1, 0), (0, 0), (1, 0), (2, 0)],
            Mino::O => [(0, 0), (1, 0), (0, 1), (1, 1)],
            Mino::J => [(-1, 0), (0, 0), (1, 0), (-1, 1)],
            Mino::L => [(-1, 0), (0, 0), (1, 0), (1, 1)],
            Mino::T => [(-1, 0), (0, 0), (1, 0), (0, 1)],
        }
    }

    pub const fn width(&self) -> u8 {
        match self {
            Mino::I => 4,
            Mino::O => 2,
            _ => 3,
        }
    }
}
