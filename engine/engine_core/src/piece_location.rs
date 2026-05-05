use crate::{piece::Mino, rotation::Rotation, spin::Spin};

#[derive(Debug, Clone)]
pub struct PieceLocation {
    pub piece: Mino,
    pub x: i8,
    pub y: i8,
    pub rotation: Rotation,
    pub spin: Spin,
}

macro_rules! lutify {
    (($e:expr) for $v:ident in [$($val:expr),*]) => {
        [$({
            let $v = $val;
            $e
        }),*]
    };
}

macro_rules! piece_lut {
    ($v:ident => $e:expr) => {
        lutify!(($e) for $v in [Mino::I, Mino::O, Mino::T, Mino::L, Mino::J, Mino::S, Mino::Z])
    };
}

macro_rules! rotation_lut {
    ($v:ident => $e:expr) => {
        lutify!(($e) for $v in [Rotation::North, Rotation::East, Rotation::South, Rotation::West])
    };
}

pub const LUT: [[[(i8, i8); 4]; 4]; 7] =
    piece_lut!(piece => rotation_lut!(rotation => rotation.rotate_blocks(piece.blocks())));

impl PieceLocation {
    pub const fn blocks(&self) -> [(i8, i8); 4] {
        self.translate_blocks(LUT[self.piece as usize][self.rotation as usize])
    }

    const fn translate(&self, (x, y): (i8, i8)) -> (i8, i8) {
        (x + self.x, y + self.y)
    }

    const fn translate_blocks(&self, cells: [(i8, i8); 4]) -> [(i8, i8); 4] {
        [
            self.translate(cells[0]),
            self.translate(cells[1]),
            self.translate(cells[2]),
            self.translate(cells[3]),
        ]
    }
}
