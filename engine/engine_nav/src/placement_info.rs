use engine_core::{piece::Mino, piece_location::PieceLocation, spin::Spin};

#[derive(Debug, Clone)]
pub struct PlacementInfo {
    pub lines_cleared: u8,
    pub lines_received: u16,
    pub pc: bool,
    pub b2b_clear: bool,
    pub broke_surge: bool,
    pub spin: Spin,
    pub outgoing_attack: u16,
    pub mino: Mino,
    pub loc: PieceLocation,
}
