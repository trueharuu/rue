use triangle::Engine;

use crate::utils;

use super::Bot;

impl Bot {
    fn board_top(engine: &Engine) -> f64 {
        let idx = engine
            .board
            .state
            .iter()
            .position(|row| row.iter().all(std::option::Option::is_none));
        let top = idx.map_or(engine.board.state.len() as i64 - 1, |i| i as i64 - 1);
        top.max(0) as f64
    }

    fn bursting(engine: &Engine, opponent: Option<&Engine>) -> Option<bool> {
        const BUFFER: f64 = 8.0;
        let multiplier = engine.dynamic.1.get();
        let board_top = Self::board_top(engine);
        let board_height = engine.board.height as f64;

        if board_top + f64::from(engine.garbage_queue.size()) * multiplier >= board_height - BUFFER
        {
            return Some(true);
        }

        if let Some(opp) = opponent {
            let opp_multiplier = opp.dynamic.1.get();
            let opp_top = Self::board_top(opp);
            let opp_height = opp.board.height as f64;

            if opp_top + f64::from(opp.garbage_queue.size()) * opp_multiplier
                >= opp_height - (BUFFER * 2.0 / 3.0)
            {
                return Some(false);
            }
        }

        None
    }

    /// Calculates the maximum burst speed multiplier based on the current pieces per second (PPS).
    fn max_burst_speed(pps: f64) -> f64 {
        (2.0 - pps.ln() / 20f64.ln()).max(1.0)
    }

    /// Calculates the burst factor based on the current engine state and optional opponent state.
    pub(super) async fn burst_factor(&self, engine: &Engine, opponent: Option<&Engine>) -> f64 {
        const BUFFER: f64 = 8.0;
        const FACTOR_DEFENSIVE: f64 = 0.3;
        const FACTOR_OFFENSIVE: f64 = 0.1;

        let is_offensive = Self::bursting(engine, opponent) == Some(false);

        let size = if is_offensive {
            if let Some(opp) = opponent {
                let opp_multiplier = opp.dynamic.1.get();
                let opp_top = Self::board_top(opp);
                let opp_height = opp.board.height as f64;
                let opp_size = f64::from(opp.garbage_queue.size());
                (opp_top * opp_multiplier + opp_size.min(20.0) * opp_multiplier
                    - 1.0
                    - (opp_height - BUFFER))
                    .max(0.0)
            } else {
                0.0
            }
        } else {
            let multiplier = engine.dynamic.1.get();
            let board_top = Self::board_top(engine);
            let board_height = engine.board.height as f64;
            let garbage_size = f64::from(engine.garbage_queue.size());
            (board_top * multiplier + garbage_size * multiplier - 1.0 - (board_height - BUFFER))
                .max(0.0)
        };

        let pps = self.config.read().await.pps;
        let factor = if is_offensive {
            FACTOR_OFFENSIVE
        } else {
            FACTOR_DEFENSIVE
        };
        (size / BUFFER * factor + 1.0).min(Self::max_burst_speed(pps))
    }

    /// Calculates the effective pieces per second (PPS) based on the current engine state and optional opponent state.
    async fn effective_pps(&self, engine: &Engine, opponent: Option<&Engine>) -> f64 {
        let pps = self.config.read().await.pps;
        if !self.config.read().await.burst {
            return pps;
        }
        match Self::bursting(engine, opponent) {
            Some(_) => pps * self.burst_factor(engine, opponent).await,
            None => pps,
        }
    }

    /// Calculates the frame at which the next piece will spawn based on the current engine state,
    /// optional next hard drop frame, and optional opponent state.
    pub(super) async fn next_piece_frame(
        &self,
        engine: &Engine,
        next_hard_drop_frame: Option<f64>,
        opponent: Option<&Engine>,
    ) -> u64 {
        const MAX_DELTA: f64 = 0.2;
        let pps = self.effective_pps(engine, opponent).await;
        let last_piece_frame = {
            let state = self.state.read().await;
            state
                .game
                .as_ref()
                .map_or(engine.frame as f64, |g| g.last_piece_frame as f64)
        };

        let frames = utils::frames_till_next_piece(
            engine.stats.pieces,
            pps,
            last_piece_frame,
            pps * (1.0 - MAX_DELTA),
            pps * (1.0 + MAX_DELTA),
        );

        let result = utils::normal_random(frames, 1.0) + last_piece_frame;
        let next_hd = next_hard_drop_frame.unwrap_or(f64::NEG_INFINITY) + 1.0;

        result.max(next_hd).max(engine.frame as f64 + 1.0) as u64
    }
}
