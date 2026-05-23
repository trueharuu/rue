use engine_core::{
    piece::{PIECE_NB, Piece}, placement::Move, ruleset::AttackContext, spin::SpinType,
};

#[derive(Clone, Copy, Debug)]
pub struct ActiveModel {
    pub waste: [f64; PIECE_NB],
    pub clear: [f64; 5],      // 0-4
    pub clear_mini: [f64; 4], // 0-3
    pub clear_spin: [f64; 4], // 0-3

    pub b2b: f64,
    pub combo: f64,

    pub in_multiplier: f64,

    pub perfect_clear: f64,
}

impl ActiveModel {
    #[must_use]
    pub fn eval(&self, mv: &Move, ctx: &AttackContext) -> f64 {
        let mut score = 0.0;

        // a piece is waste if it clears 0 lines
        // however, an I piece is waste if it is not quad
        if ctx.lines == 0 || (mv.piece() == Piece::I && ctx.lines < 4) {
            score += self.waste[mv.piece() as usize];
        }

        match mv.spin() {
            SpinType::NoSpin => score += self.clear[ctx.lines as usize],
            SpinType::Mini => score += self.clear_mini[ctx.lines as usize],
            SpinType::Full => score += self.clear_spin[ctx.lines as usize],
        }

        let b2b = ctx.b2b.min(8);
        score += self.b2b * f64::from(b2b);

        score += self.combo * f64::from(ctx.combo);

        score += self.perfect_clear * f64::from(ctx.is_perfect_clear);

        let is_special_clear = ctx.lines == 4 || ctx.spin == SpinType::Full;
        if is_special_clear && ctx.combo > 2 {
            score += self.in_multiplier * f64::from(ctx.combo);
        }

        score
    }
}
