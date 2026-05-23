use engine_core::{piece::PIECE_NB, placement::Move};

#[derive(Clone, Copy)]
pub struct ActiveModel {
    pub waste: [f64; PIECE_NB],
}

impl ActiveModel {
    #[must_use]
    pub fn eval(&self, mv: &Move) -> f64 {
        let mut score = 0.0;
        score += self.waste[mv.piece() as usize];
        score
    }
}
