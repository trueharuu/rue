use engine_core::game::Game;

#[derive(Clone, Copy)]
pub struct OpponentModel {
}

impl OpponentModel {
    #[must_use]
    pub fn eval(&self, _game: &Game) -> f64 {
        let mut score = 0.0;
        score += 0.0;
        score
    }
}