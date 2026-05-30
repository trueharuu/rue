use rue_core::{board::{Bitboard, COL_NB}, data::{canonical_r, canonical_size, in_bounds}, piece::Piece, placement::piece_table, rotation::{ROTATION_NB, Rotation}};

pub struct CollisionMap {
    pub board: [[Bitboard; 4]; COL_NB],
}

impl CollisionMap {
    pub fn new(cols: &[Bitboard; COL_NB], p: Piece) -> Self {
        let cs = canonical_size(p);
        let mut board = [[0u64; 4]; COL_NB];

        for x in 0..COL_NB as i32 {
            for (ri, entry) in board[x as usize].iter_mut().enumerate().take(cs) {
                let r: Rotation = Rotation::from_u8(ri as u8);
                if !in_bounds(p, r, x) {
                    *entry = !0u64;
                    continue;
                }
                let pc = piece_table(p, r);
                let mut result = cols[x as usize];
                for k in 0..3 {
                    let cx = x + pc[k].x as i32;
                    let cy = pc[k].y as i32;
                    if cy < 0 {
                        result |= !((!cols[cx as usize]) << ((-cy) as u32));
                    } else {
                        result |= cols[cx as usize] >> (cy as u32);
                    }
                }
                *entry = result;
            }
        }

        CollisionMap { board }
    }

    pub fn get(&self, x: usize, r: Rotation) -> Bitboard {
        self.board[x][r as usize]
    }
}

pub struct CollisionMap16 {
    pub board: [Bitboard; COL_NB],
}

impl CollisionMap16 {
    pub fn new(cols: &[Bitboard; COL_NB], p: Piece) -> Self {
        let mut board = [0u64; COL_NB];

        for x in 0..COL_NB as i32 {
            let mut val: Bitboard = 0;
            for ri in 0..ROTATION_NB as u8 {
                let r: Rotation = Rotation::from_u8(ri);
                let rr = canonical_r(p, r);

                let lane = if !in_bounds(p, rr, x) {
                    0xFFFFu64
                } else {
                    let pc = piece_table(p, rr);
                    let mut result = cols[x as usize];
                    for k in 0..3 {
                        let cx = x + pc[k].x as i32;
                        let cy = pc[k].y as i32;
                        if cy < 0 {
                            result |= !((!cols[cx as usize]) << ((-cy) as u32));
                        } else {
                            result |= cols[cx as usize] >> (cy as u32);
                        }
                    }
                    result & 0xFFFFu64
                };

                val |= lane << (ri as u32 * 16);
            }
            board[x as usize] = val;
        }

        CollisionMap16 { board }
    }

    pub fn get(&self, x: usize) -> Bitboard {
        self.board[x]
    }
}