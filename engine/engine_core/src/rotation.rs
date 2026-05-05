pub const ROT: [Rotation; 4] = [
    Rotation::North,
    Rotation::East,
    Rotation::South,
    Rotation::West,
];

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Rotation {
    North,
    East,
    South,
    West,
}

impl Rotation {
    pub const fn canonical(&self) -> Rotation {
        match self {
            Rotation::North => Rotation::North,
            Rotation::East => Rotation::East,
            Rotation::South => Rotation::North,
            Rotation::West => Rotation::East,
        }
    }
    pub const fn rotate_block(&self, (x, y): (i8, i8)) -> (i8, i8) {
        match self {
            Rotation::North => (x, y),
            Rotation::East => (y, -x),
            Rotation::South => (-x, -y),
            Rotation::West => (-y, x),
        }
    }

    pub const fn rotate_blocks(&self, blocks: [(i8, i8); 4]) -> [(i8, i8); 4] {
        [
            self.rotate_block(blocks[0]),
            self.rotate_block(blocks[1]),
            self.rotate_block(blocks[2]),
            self.rotate_block(blocks[3]),
        ]
    }

    pub const fn rotate_cw(&self) -> Rotation {
        match self {
            Rotation::North => Rotation::East,
            Rotation::East => Rotation::South,
            Rotation::South => Rotation::West,
            Rotation::West => Rotation::North,
        }
    }

    pub const fn rotate_ccw(&self) -> Rotation {
        match self {
            Rotation::North => Rotation::West,
            Rotation::West => Rotation::South,
            Rotation::South => Rotation::East,
            Rotation::East => Rotation::North,
        }
    }

    pub const fn rotate_180(&self) -> Rotation {
        match self {
            Rotation::North => Rotation::South,
            Rotation::East => Rotation::West,
            Rotation::South => Rotation::North,
            Rotation::West => Rotation::East,
        }
    }
}
