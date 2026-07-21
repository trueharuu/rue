mod settings;
mod utils;
use std::{fmt, sync::Arc};

use tokio::sync::{Mutex, RwLock};

use triangle::{
    Client, ClientOptions, Engine,
    classes::ribbon,
    types::{
        events::recv,
        game::{Key, tick},
        room::Bracket,
    },
    utils::{EventEmitter, api::core::ApiError, events::WrapError},
};

use triangle::engine::queue::bag::BagType;

use rue_core::{
    board::Board,
    game::{Game, garbage::GarbageQueue, ruleset::SEASON_2},
    piece::Piece,
    rng::{Rng, RngKind},
};
use rue_eval::simple::Simple;
use rue_nav::pathfinder;
use rue_search::{SearchConfig, beam_search};

use crate::{
    command::{self, core::{registry::Registry, traits::{Restriction, User}}},
    util::{config::CONFIG, env::env, events::{events, msgs}},
};

use settings::{ConstraintLevel, SettingsHandler};
use utils::BotMove;

/// Number of 6-row bands backing the live game board (42 rows).
const BOARD_BANDS: usize = 7;
/// The persistent solver-side game state kept for the live room.
type BotGame = Game<BOARD_BANDS>;

/// Chat command prefix.
const PREFIX: &str = ">";
/// Bot name, used in reply/restriction messages.
const BOT_NAME: &str = "rue";
/// Beam search depth (in placements) used for real-time move selection.
const SEARCH_DEPTH: usize = 6;
/// Beam width used for real-time move selection.
const SEARCH_BEAM_WIDTH: usize = 300;
/// Minimum queue length to keep buffered ahead of the search depth.
const QUEUE_LOOKAHEAD: usize = SEARCH_DEPTH + 7;

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

    pub fn max(&self, other: FrameCounter) -> Self {
        Self(self.0.max(other.0))
    }
}

#[derive(Debug, Clone)]
pub enum Target {
    Join(String),
    Create,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Finesse {
    Instant,
    Smooth,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub pps: f64,
    pub burst: bool,
    pub finesse: Finesse,
}

#[derive(Debug, Clone)]
pub struct EnabledState {
    pub value: bool,
    pub attempt: bool,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct GameState {
    last_piece_frame: u64,
    target_frame: u64,
}

#[derive(Debug, Clone)]
pub struct State {
    pub enabled: EnabledState,
    game: Option<GameState>,
    pub restriction: Restriction,
}

pub struct Bot {
    game: Mutex<BotGame>,
    weights: Simple,
    pub client: Client,
    pub config: RwLock<Config>,
    pub state: RwLock<State>,
    pub settings: SettingsHandler,
    events: EventEmitter,
    pub registry: Registry,
}

#[derive(Debug)]
pub enum BotError {
    ConnectionError(ApiError),
    RoomError(WrapError),
    IoError(std::io::Error),
}

impl fmt::Display for BotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BotError::ConnectionError(err) => write!(f, "Failed to create client: {err}"),
            BotError::RoomError(err) => write!(f, "Failed to join or create room: {err}"),
            BotError::IoError(err) => write!(f, "IO error: {err}"),
        }
    }
}

impl std::error::Error for BotError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BotError::ConnectionError(err) => Some(err),
            BotError::RoomError(err) => Some(err),
            BotError::IoError(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for BotError {
    fn from(err: std::io::Error) -> Self {
        BotError::IoError(err)
    }
}

/// Appends `n` shuffled 7-bags to the end of the queue.
fn fill(queue: &mut Vec<Piece>, rng: &mut Rng, n: usize) {
    for _ in 0..n {
        let mut slice = RngKind::Bag7.slice();
        rng.shuffle_array(&mut slice);
        queue.extend_from_slice(&slice);
    }
}

impl Bot {
    pub async fn new(target: Target) -> Result<Arc<Self>, BotError> {
        let client = Client::new(ClientOptions {
            game: Some(triangle::classes::GameOptions {
                handling: Some(CONFIG.handling),
                spectating_strategy: None,
            }),
            ribbon: Some(ribbon::OptionalParams {
                options: Some(ribbon::Options {
                    logging: ribbon::LoggingLevel::Error,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            social: None,
            token: triangle::Credentials::Token(env().token.clone()),
            user_agent: None,
        })
        .await
        .map_err(BotError::ConnectionError)?;

        let (room_tx, room_rx) = tokio::sync::oneshot::channel::<recv::room::Update>();
        let room_tx = Arc::new(Mutex::new(Some(room_tx)));

        client.on::<recv::room::Update>(async move |data| {
            if let Some(tx) = room_tx.lock().await.take() {
                tx.send(data).ok();
            }
        });

        match target {
            Target::Join(roomid) => client.join_room(&roomid).await,
            Target::Create => client.create_room(false).await,
        }
        .map_err(BotError::RoomError)?;

        let room_update_data = room_rx
            .await
            .map_err(|_| BotError::RoomError(WrapError::ServerError))?;

        let mut registry = Registry::new();
        registry.register(Box::new(command::info::ping_command));
        registry.register(Box::new(command::info::help_command));
        registry.register(Box::new(command::controls::kill_command));
        registry.register(Box::new(command::controls::enable_command));
        registry.register(Box::new(command::controls::disable_command));
        registry.register(Box::new(command::controls::restrict_command));
        registry.register(Box::new(command::controls::pps_command));
        registry.register(Box::new(command::controls::burst_command));
        registry.register(Box::new(command::controls::finesse_command));

        let weights =
            serde_json::from_str::<Simple>(&std::fs::read_to_string(env().weights.clone())?)
                .map_err(|e| BotError::IoError(e.into()))?;

        let bot = Arc::new(Bot {
            // Real seeding happens once the room's queue seed is known, on round start.
            game: Mutex::new(Game {
                board: Board::EMPTY,
                hold: None,
                queue: Vec::new(),
                garbage_queue: GarbageQueue::new(),
                b2b_count: None,
                combo_count: None,
                ruleset: SEASON_2,
                rng: Rng::new(),
            }),
            weights,
            client,
            settings: SettingsHandler::new(),
            config: RwLock::new(Config {
                finesse: Finesse::Smooth,
                pps: 1.0,
                burst: true,
            }),
            state: RwLock::new(State {
                enabled: EnabledState {
                    value: false,
                    attempt: true,
                    force: false,
                },
                game: None,
                restriction: Restriction::None,
            }),
            events: EventEmitter::new(),
            registry,
        });

        bot.handle_room_update(room_update_data, true).await;

        if let Some(room) = bot.client.room() {
            room.chat(":oyes:/").await.ok();
        } else {
            return Err(BotError::RoomError(WrapError::ServerError));
        }

        bot.bind().await;

        Ok(bot)
    }

    async fn bind(self: &Arc<Self>) {
        let b = self.clone();
        events()
            .on::<msgs::Shutdown>(async move |_| {
                let b = b.clone();
                b.destroy().await;
            })
            .await;

        let b = self.clone();

        self.client.on::<recv::client::Dead>(async move |_| {
            b.destroy().await;
        });

        let b = self.clone();

        self.client.on::<recv::room::Leave>(async move |_| {
            b.destroy().await;
        });

        let b = self.clone();

        self.client.on::<recv::room::Update>(async move |data| {
            b.handle_room_update(data, false).await;
        });

        let b = self.clone();

        self.client
            .on::<recv::client::game::round::End>(async move |_| {
                b.state.write().await.game = None;
            });

        let b = self.clone();

        self.client.on::<recv::room::Chat>(async move |data| {
            if data.system || data.user.id.is_none() {
                return;
            }
            if data
                .user
                .id
                .as_ref()
                .map_or(false, |id| *id == b.client.user.id)
            {
                return;
            }

            let bot_username = b.client.user.username.clone();

            if data.content == format!("@{}", bot_username) {
                if let Some(room) = b.client.room() {
                    room.chat(&format!("My prefix is {PREFIX}")).await.ok();
                }
                return;
            }

            let content = if data.content.starts_with(&format!("@{} ", bot_username)) {
                data.content
                    .replacen(&format!("@{} ", bot_username), PREFIX, 1)
            } else {
                data.content.clone()
            }
            .to_lowercase();

            let Some(rest) = content.strip_prefix(PREFIX) else {
                return;
            };

            let mut parts = rest.splitn(2, char::is_whitespace);
            let cmd_name = parts.next().unwrap_or("");
            if cmd_name.is_empty() {
                return;
            }
            let args_text = parts.next().unwrap_or("");

            let user_id = data.user.id.as_deref().unwrap_or("").to_string();
            let room_info = b.client.room().map(|r| {
                let s = r.state.lock();
                (s.owner.clone(), s.players.clone())
            });

            let level = if let Some((owner, players)) = &room_info {
                if user_id == *owner {
                    Restriction::Host
                } else if players
                    .iter()
                    .any(|p| matches!(p.bracket, Bracket::Player) && p.id == user_id)
                {
                    Restriction::Player
                } else {
                    Restriction::None
                }
            } else {
                Restriction::None
            };

            let user = User {
                id: user_id,
                name: data.user.username.clone(),
                level,
            };

            let Some(cmd) = b.registry.find(cmd_name) else {
                if let Some(room) = b.client.room() {
                    room.chat(&format!(
                        "Unknown command.\nRun {PREFIX}help for a list of valid commands."
                    ))
                    .await
                    .ok();
                }
                return;
            };

            let meta = cmd.metadata();
            let restriction = b.state.read().await.restriction;
            if user.level < meta.restriction_level || user.level < restriction {
                if let Some(room) = b.client.room() {
                    room.chat(&format!("{BOT_NAME}'s commands are currently restricted."))
                        .await
                        .ok();
                }
                return;
            }

            let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(8);
            let b_replies = b.clone();
            tokio::spawn(async move {
                while let Some(message) = rx.recv().await {
                    if let Some(room) = b_replies.client.room() {
                        room.chat(&message).await.ok();
                    }
                }
            });

            let mut ctx = crate::command::core::context::Context::new(
                args_text,
                &tx,
                b.clone(),
                user,
            );
            cmd.execute(&mut ctx).await.ok();
        });

        // this.client.on("client.room.players", (players) => {
        //   if (players.every((p) => p.bot)) this.destroy();
        // });

        let b = self.clone();

        self.client
            .on::<recv::client::room::Players>(async move |data| {
                if data.0.iter().all(|p| p.bot) {
                    b.destroy().await;
                }
            });

        let b = self.clone();
        self.client
            .on::<recv::client::game::Start>(async move |data| {
                if data.players.iter().any(|p| p.0 == b.client.user.id) {
                    if let Some(room) = b.client.room() {
                        room.chat("glhf!").await.ok();
                    }
                }
            });

        let b = self.clone();

        self.client
            .on::<recv::client::game::round::Start>(async move |_| {
                let engine_snap = b
                    .client
                    .game()
                    .and_then(|g| g.me)
                    .map(|me| me.state.lock().engine.clone());

                let engine = match engine_snap {
                    Some(e) => e,
                    None => return,
                };

                b.client.game().unwrap().me.unwrap().set_pause_iges(true);

                if !matches!(engine.queue.kind, BagType::Bag7) {
                    eprintln!("Unsupported bag type: {:?}", engine.queue.kind);
                    return;
                }

                {
                    let mut rng = Rng::new_seeded(engine.queue.seed as i32);
                    let mut queue = Vec::new();
                    fill(&mut queue, &mut rng, QUEUE_LOOKAHEAD.div_ceil(7).max(1));

                    let mut game = b.game.lock().await;
                    *game = Game {
                        board: Board::EMPTY,
                        hold: None,
                        queue,
                        garbage_queue: GarbageQueue::new(),
                        b2b_count: None,
                        combo_count: None,
                        ruleset: SEASON_2,
                        rng,
                    };
                }

                {
                    let mut state = b.state.write().await;
                    state.game = Some(GameState {
                        last_piece_frame: 0,
                        target_frame: 0,
                    });
                    drop(state);
                    let target_frame = b.next_piece_frame(&engine, None, None).await;
                    b.state.write().await.game = Some(GameState {
                        last_piece_frame: engine.frame,
                        target_frame,
                    });
                }

                let b2 = b.clone();
                b.client
                    .register_ticker(move |input| {
                        let b = b2.clone();
                        Box::pin(async move { b.tick(input).await })
                    })
                    .await
                    .ok();
            });
    }

    async fn handle_room_update(self: &Arc<Self>, data: recv::room::Update, initial: bool) {
        let result = self.settings.check_room_update(&data);

        if let Some(result) = &result {
            if let Some(room) = self.client.room() {
                for output in &result.outputs {
                    room.chat(&format!(
                        "{}: {}",
                        output.level.to_string().to_uppercase(),
                        output.message
                    ))
                    .await
                    .ok();
                }
            }
            if result.level == ConstraintLevel::Error {
                if let Some(mut room) = self.client.room() {
                    room.switch(Bracket::Spectator).await.ok();
                }
                {
                    let mut state = self.state.write().await;
                    state.enabled.attempt = true;
                    state.enabled.value = false;
                }

                if initial
                    && result
                        .outputs
                        .iter()
                        .any(|o| o.message == "rue requires 0 gravity increase.")
                    && result
                        .outputs
                        .iter()
                        .any(|o| o.message == "rue requires 0 gravity.")
                {
                    if let Some(room) = self.client.room() {
                        room.chat("Paste:\n\n/set options.g=0;options.gincrease=0;\n\nin chat and press enter to enable rue.").await.ok();
                    }
                }

                return;
            }
        }
        let attempt = self.state.read().await.enabled.attempt;
        if result
            .as_ref()
            .map_or(true, |r| r.level != ConstraintLevel::Error)
            && attempt
        {
            if let Some(mut room) = self.client.room() {
                room.switch(Bracket::Player).await.ok();
            }
            self.state.write().await.enabled.value = true;
        }
    }

    fn board_top(engine: &Engine) -> f64 {
        let idx = engine
            .board
            .state
            .iter()
            .position(|row| row.iter().all(|cell| cell.is_none()));
        let top = idx
            .map(|i| i as i64 - 1)
            .unwrap_or(engine.board.state.len() as i64 - 1);
        top.max(0) as f64
    }

    fn bursting(engine: &Engine, opponent: Option<&Engine>) -> Option<bool> {
        const BUFFER: f64 = 8.0;
        let multiplier = engine.dynamic.1.get();
        let board_top = Self::board_top(engine);
        let board_height = engine.board.height as f64;

        if board_top + engine.garbage_queue.size() as f64 * multiplier >= board_height - BUFFER {
            return Some(true);
        }

        if let Some(opp) = opponent {
            let opp_multiplier = opp.dynamic.1.get();
            let opp_top = Self::board_top(opp);
            let opp_height = opp.board.height as f64;

            if opp_top + opp.garbage_queue.size() as f64 * opp_multiplier
                >= opp_height - (BUFFER * 2.0 / 3.0)
            {
                return Some(false);
            }
        }

        None
    }

    fn max_burst_speed(pps: f64) -> f64 {
        (2.0 - pps.ln() / 20f64.ln()).max(1.0)
    }

    async fn burst_factor(&self, engine: &Engine, opponent: Option<&Engine>) -> f64 {
        const BUFFER: f64 = 8.0;
        const FACTOR_DEFENSIVE: f64 = 0.3;
        const FACTOR_OFFENSIVE: f64 = 0.1;

        let is_offensive = Self::bursting(engine, opponent) == Some(false);

        let size = if is_offensive {
            if let Some(opp) = opponent {
                let opp_multiplier = opp.dynamic.1.get();
                let opp_top = Self::board_top(opp);
                let opp_height = opp.board.height as f64;
                let opp_size = opp.garbage_queue.size() as f64;
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
            let garbage_size = engine.garbage_queue.size() as f64;
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

    async fn next_piece_frame(
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

    fn keypress_duration(&self, m: &BotMove, engine: &Engine) -> f64 {
        if matches!(
            m,
            BotMove::Path(pathfinder::Input::SoftDrop | pathfinder::Input::SonicDrop)
        ) {
            0.1
        } else if matches!(
            m,
            BotMove::Path(pathfinder::Input::DasLeft | pathfinder::Input::DasRight)
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
                        let duration = self.keypress_duration(m, engine);
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
                    .filter(|m| {
                        matches!(
                            m,
                            BotMove::Path(pathfinder::Input::SoftDrop | pathfinder::Input::SonicDrop)
                        )
                    })
                    .count();
                let das_count = raw
                    .iter()
                    .filter(|m| {
                        matches!(
                            m,
                            BotMove::Path(pathfinder::Input::DasLeft | pathfinder::Input::DasRight)
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
                        BotMove::Path(pathfinder::Input::DasLeft | pathfinder::Input::DasRight)
                    );
                    let arr_time = if is_das {
                        let x_before = sim_falling.x();
                        if matches!(m, BotMove::Path(pathfinder::Input::DasLeft)) {
                            sim_falling.das_left(&engine.board.state);
                        } else {
                            sim_falling.das_right(&engine.board.state);
                        }
                        let displacement = (sim_falling.x() - x_before).abs() as f64;
                        (arr * (displacement - 1.0)).max(0.0)
                    } else {
                        match m {
                            BotMove::Path(pathfinder::Input::RotateCW) => {
                                sim_falling.set_rotation(sim_falling.rotation() as i32 + 1)
                            }
                            BotMove::Path(pathfinder::Input::RotateCCW) => {
                                sim_falling.set_rotation(sim_falling.rotation() as i32 - 1)
                            }
                            BotMove::Path(pathfinder::Input::RotateFlip) => {
                                sim_falling.set_rotation(sim_falling.rotation() as i32 + 2)
                            }
                            _ => {}
                        }
                        0.0
                    };

                    let duration = self.keypress_duration(m, engine) + arr_time;

                    tmp.push((*m, frame.as_f64(), duration, delay));

                    let prev_frame = frame.0;
                    frame.add(delay + duration);

                    if matches!(
                        m,
                        BotMove::Path(pathfinder::Input::SoftDrop | pathfinder::Input::SonicDrop)
                    ) && frame.as_f64() != 0.0
                    {
                        frame = frame.max(FrameCounter((prev_frame + duration).ceil()));
                    }
                }

                let total: f64 = tmp.iter().map(|(_, _, d, delay)| delay + d).sum();
                if total > time_to_next as f64 {
                    let duration_sum: f64 = tmp.iter().map(|(_, _, d, _)| d).sum();
                    let multiplier = (time_to_next as f64 + duration_sum) / total;
                    tmp.iter_mut()
                        .for_each(|(_, _, _, delay)| *delay *= multiplier);
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

    async fn tick(&self, input: tick::In) -> tick::Out {
        if !input.new_garbage.is_empty() {
            let mut game = self.game.lock().await;
            let cap = game.ruleset.garbage_absolute_cap;
            for g in &input.new_garbage {
                game.garbage_queue.recieve(g.amount, cap);
            }
        }

        let game_state = { self.state.read().await.game.as_ref().map(|g| g.target_frame) };

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

        let has_hard_drop = self
            .client
            .game()
            .and_then(|g| g.me)
            .map(|me| {
                me.state
                    .lock()
                    .key_queue
                    .iter()
                    .any(|kp| kp.data.key == Key::HardDrop)
            })
            .unwrap_or(false);

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

        let opponent_engine = self
            .client
            .game()
            .map(|g| {
                g.state
                    .lock()
                    .players
                    .iter()
                    .find(|p| p.userid != self.client.user.id.as_str())
                    .map(|p| p.state.lock().engine.clone())
            })
            .flatten();

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
            if game.queue.len() < QUEUE_LOOKAHEAD {
                fill(&mut game.queue, &mut game.rng, 2);
            }

            let cfg = SearchConfig {
                beam_width: SEARCH_BEAM_WIDTH,
                depth: SEARCH_DEPTH,
                futility_delta: 0.0,
                ..SearchConfig::default()
            };

            match beam_search(&game, &cfg, &self.weights) {
                Some(result) => {
                    let mv = result.best.root_move;
                    let requires_hold = mv.piece() != game.queue[0];
                    let inputs = pathfinder::get_input(&game.board, mv, &game.ruleset, true, false);
                    game.tick(mv);

                    let mut raw = Vec::with_capacity(inputs.0.len() + 1);
                    if requires_hold {
                        raw.push(BotMove::Hold);
                    }
                    raw.extend(inputs.0.into_iter().map(BotMove::Path));
                    raw
                }
                None => Vec::new(),
            }
        };

        let keys = self
            .process_keys(&raw_keys, &input.engine, opponent_engine.as_ref())
            .await;

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

    pub async fn destroy(&self) {
        self.client.destroy().await;

        self.events.emit_raw("close", serde_json::json!({}));
        self.events.destroy();
    }
}
