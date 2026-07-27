use rue_core::game::ruleset::SEASON_2_HANDLING;
use rue_core::render;
use rue_nav::pathfinder::input::Input;
use triangle::Engine;
use triangle::types::game::Key;
use triangle::types::game::tick;

use rue_nav::pathfinder;
use rue_search::SearchConfig;
use rue_search::beam_search;

use crate::bot::state::Finesse;
use crate::utils::BotMove;
use crate::utils::{self};

use super::Bot;
use super::fill;

/// A strictly-increasing frame counter that can represent subframes as tenths of a frame.
struct FrameCounter(f64);
impl FrameCounter {
    pub fn new(v: u64) -> Self {
        Self(v as f64)
    }

    pub fn add(&mut self, delta: f64) {
        self.0 = ((self.0 + delta) * 10.0).round() / 10.0;
    }

    pub fn frame(&self) -> u64 {
        self.0.floor() as u64
    }

    pub fn subframe(&self) -> f64 {
        ((self.0 - self.0.floor()) * 10.0).round() / 10.0
    }

    pub fn as_f64(&self) -> f64 {
        (self.0 * 10.0).round() / 10.0
    }

    pub fn max(&self, other: &FrameCounter) -> Self {
        Self(self.0.max(other.0))
    }
}

impl Bot {
    fn keypress_duration(m: BotMove, engine: &Engine) -> f64 {
        if matches!(m, BotMove::Path(Input::SoftDrop)) {
            0.1
        } else if matches!(
            m,
            BotMove::Path(Input::DasLeft | Input::DasRight)
        ) {
            engine.handling.das + 0.1
        } else {
            0.0
        }
    }

    async fn process_keys(
        &self,
        raw: &[BotMove],
        engine: &Engine,
        opponent: Option<&Engine>,
    ) -> Vec<tick::Keypress> {
        struct InternalKeypress {
            key: Key,
            frame: f64,
            duration: f64,
        }

        let now = engine.frame;

        let finesse = self.config.read().await.finesse;
        let frames: Vec<InternalKeypress> = match finesse {
            Finesse::Instant => {
                let mut frame = FrameCounter::new(now);
                raw.iter()
                    .map(|m| {
                        let duration = Self::keypress_duration(*m, engine);
                        let kp = InternalKeypress {
                            key: utils::move_to_key(*m),
                            frame: frame.as_f64(),
                            duration,
                        };
                        frame.add(duration);
                        kp
                    })
                    .collect()
            }

            Finesse::Smooth => {
                const MAX_PIECE_FRAMES: u64 = 45;

                let mut frame = FrameCounter::new(now);
                let time_to_next = (self
                    .next_piece_frame(engine, None, opponent)
                    .await
                    .saturating_sub(now)
                    .saturating_sub(1))
                .min(MAX_PIECE_FRAMES);

                let arr = engine.handling.arr;

                let soft_drop_count = raw
                    .iter()
                    .filter(|m| matches!(m, BotMove::Path(Input::SoftDrop)))
                    .count();
                let das_count = raw
                    .iter()
                    .filter(|m| {
                        matches!(
                            m,
                            BotMove::Path(Input::DasLeft | Input::DasRight)
                        )
                    })
                    .count();
                let time_per_press = ((time_to_next as f64
                    - soft_drop_count as f64 * 0.1
                    - das_count as f64 * engine.handling.das)
                    / raw.len() as f64)
                    * 0.99;

                let mut sim_falling = engine.falling.clone();

                // key, frame, duration, delay
                let mut tmp: Vec<(BotMove, f64, f64, f64)> = Vec::new();

                for m in raw {
                    let delay = time_per_press.max(0.0);
                    let is_das = matches!(
                        m,
                        BotMove::Path(Input::DasLeft | Input::DasRight)
                    );
                    let arr_time = if is_das {
                        let x_before = sim_falling.x();
                        if matches!(m, BotMove::Path(Input::DasLeft)) {
                            sim_falling.das_left(&engine.board.state);
                        } else {
                            sim_falling.das_right(&engine.board.state);
                        }
                        let displacement = f64::from((sim_falling.x() - x_before).abs());
                        (arr * (displacement - 1.0)).max(0.0)
                    } else {
                        match m {
                            BotMove::Path(Input::RotateCW) => {
                                sim_falling.set_rotation(i32::from(sim_falling.rotation()) + 1);
                            }
                            BotMove::Path(Input::RotateCCW) => {
                                sim_falling.set_rotation(i32::from(sim_falling.rotation()) - 1);
                            }
                            BotMove::Path(Input::RotateFlip) => {
                                sim_falling.set_rotation(i32::from(sim_falling.rotation()) + 2);
                            }
                            _ => {}
                        }
                        0.0
                    };

                    let duration = Self::keypress_duration(*m, engine) + arr_time;

                    tmp.push((*m, frame.as_f64(), duration, delay));

                    let prev_frame = frame.0;
                    frame.add(delay + duration);

                    if matches!(m, BotMove::Path(Input::SoftDrop))
                        && frame.as_f64() != 0.0
                    {
                        frame = frame.max(&FrameCounter((prev_frame + duration).ceil()));
                    }
                }

                let total: f64 = tmp.iter().map(|(_, _, d, delay)| delay + d).sum();
                if total > time_to_next as f64 {
                    let duration_sum: f64 = tmp.iter().map(|(_, _, d, _)| d).sum();
                    let multiplier = (time_to_next as f64 + duration_sum) / total;
                    for (_, _, _, delay) in &mut tmp {
                        *delay *= multiplier;
                    }
                }

                tmp.into_iter()
                    .map(|(m, f, d, _)| InternalKeypress {
                        key: utils::move_to_key(m),
                        frame: f,
                        duration: d,
                    })
                    .collect()
            }
        };

        frames
            .into_iter()
            .flat_map(|f| {
                let mut frame = FrameCounter(f.frame);
                frame.add(0.0);

                let first = tick::Keypress {
                    r#type: tick::KeypressType::Keydown,
                    frame: frame.frame(),
                    data: tick::KeypressData {
                        key: f.key,
                        subframe: frame.subframe(),
                        hoisted: false,
                    },
                };

                frame.add(f.duration);

                [
                    first,
                    tick::Keypress {
                        r#type: tick::KeypressType::Keyup,
                        frame: frame.frame(),
                        data: tick::KeypressData {
                            key: f.key,
                            subframe: frame.subframe(),
                            hoisted: false,
                        },
                    },
                ]
            })
            .map(|mut kp| {
                while kp.data.subframe >= 1.0 {
                    kp.data.subframe -= 1.0;
                    kp.frame += 1;
                }
                kp.data.subframe = (kp.data.subframe * 10.0).round() / 10.0;

                kp
            })
            .collect()
    }

    pub(super) async fn tick(&self, input: tick::In) -> tick::Out {
        if !input.new_garbage.is_empty() {
            let mut game = self.game.lock().await;
            let cap = game.ruleset.garbage_absolute_cap;
            for g in &input.new_garbage {
                game.garbage_queue.recieve(g.amount, cap);
            }
        }

        let game_state = {
            self.state
                .read()
                .await
                .game
                .as_ref()
                .map(|g| g.target_frame)
        };

        let Some(target_frame) = game_state else {
            return tick::Out {
                keys: vec![],
                run_after: vec![],
            };
        };

        if input.engine.frame < target_frame {
            return tick::Out {
                keys: vec![],
                run_after: vec![],
            };
        }

        let has_hard_drop = self.client.game().and_then(|g| g.me).is_some_and(|me| {
            me.state
                .lock()
                .key_queue
                .iter()
                .any(|kp| kp.data.key == Key::HardDrop)
        });

        if has_hard_drop {
            return tick::Out {
                keys: vec![],
                run_after: vec![],
            };
        }

        {
            let mut state = self.state.write().await;
            if let Some(game) = &mut state.game {
                game.last_piece_frame = input.engine.frame;
            }
        }

        let opponent_engine = self.client.game().and_then(|g| {
            g.state
                .lock()
                .players
                .iter()
                .find(|p| p.userid != self.client.user.id.as_str())
                .map(|p| p.state.lock().engine.clone())
        });

        let initial_target = self
            .next_piece_frame(&input.engine, None, opponent_engine.as_ref())
            .await;
        {
            let mut state = self.state.write().await;
            if let Some(game) = &mut state.game {
                game.target_frame = initial_target;
            }
        }

        let raw_keys: Vec<BotMove> = {
            let mut game = self.game.lock().await;
            let game = &mut *game;
            if game.queue.len() < self.global_config.queue_buffer {
                fill(&mut game.queue, &mut game.rng, 2);
            }

            let cfg = SearchConfig {
                beam_width: self.global_config.search_beam_width,
                depth: self.config.read().await.vision,
                futility_delta: 0.0,
                ..SearchConfig::default()
            };

            match beam_search::<_, { SEASON_2_HANDLING }, _>(game, &cfg, &self.weights) {
                Some(result) => {
                    let mv = result.best.root_move;

                    let requires_hold = mv.piece() != game.queue[0];
                    let inputs =
                        pathfinder::get_input::<_, { SEASON_2_HANDLING }>(&game.board, mv);
                    game.tick(mv);

                    let mut raw = Vec::with_capacity(inputs.len() + 1);
                    if requires_hold {
                        raw.push(BotMove::Hold);
                    }
                    raw.extend(inputs.into_iter().map(BotMove::Path));
                    raw
                }
                None => Vec::new(),
            }
        };

        println!("attempting: {raw_keys:?}");
        let keys = self
            .process_keys(&raw_keys, &input.engine, opponent_engine.as_ref())
            .await;
        // println!("got: {keys:?}");

        let hd_frame = keys
            .iter()
            .rev()
            .find(|kp| kp.data.key == Key::HardDrop)
            .map(|kp| kp.frame as f64);

        let final_target = self
            .next_piece_frame(&input.engine, hd_frame, opponent_engine.as_ref())
            .await;
        {
            let mut state = self.state.write().await;
            if let Some(game) = &mut state.game {
                game.target_frame = final_target;
            }
        }

        tick::Out {
            keys,
            run_after: vec![],
        }
    }
}
