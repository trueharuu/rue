use std::sync::Arc;

use triangle::engine::queue::bag::BagType;
use triangle::types::events::recv;
use triangle::types::room::Bracket;

use crate::bot::state::GameState;
use crate::command::context::Context;
use crate::command::traits::Restriction;
use crate::command::traits::User;
use crate::events::events;
use crate::events::msgs;
use crate::settings::ConstraintLevel;
use crate::utils::FAILURE;
use crate::utils::WARNING;

use super::Bot;
impl Bot {
    pub(super) async fn bind(self: &Arc<Self>) {
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
                .is_some_and(|id| *id == b.client.user.id)
            {
                return;
            }

            let bot_username = b.client.user.username.clone();

            if data.content == format!("@{bot_username}") {
                if let Some(room) = b.client.room() {
                    room.chat(&format!("My prefix is {}", b.global_config.prefix))
                        .await
                        .ok();
                }
                return;
            }

            let content = if data.content.starts_with(&format!("@{bot_username} ")) {
                data.content.replacen(
                    &format!("@{bot_username} "),
                    b.global_config.prefix.as_str(),
                    1,
                )
            } else {
                data.content.clone()
            }
            .to_lowercase();

            let Some(rest) = content.strip_prefix(b.global_config.prefix.as_str()) else {
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
                return;
            };

            let meta = cmd.metadata();
            let restriction = b.state.read().await.restriction;
            if user.level < meta.restriction_level || user.level < restriction {
                if let Some(room) = b.client.room() {
                    room.chat(&format!(
                        "{FAILURE} commands are currently restricted to {restriction:?}"
                    ))
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

            let mut ctx = Context::new(args_text, &tx, b.clone(), user);
            cmd.execute(&mut ctx).await.ok();
        });

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
                if data.players.iter().any(|p| p.0 == b.client.user.id)
                    && let Some(room) = b.client.room()
                {
                    room.chat("glhf").await.ok();
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

                let Some(engine) = engine_snap else { return };

                b.client.game().unwrap().me.unwrap().set_pause_iges(true);

                if !matches!(engine.queue.kind, BagType::Bag7) {
                    eprintln!("unsupported bag type: {:?}", engine.queue.kind);
                    return;
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

    /// Handles a single room update, checking constraints and updating the bot's enabled state accordingly.
    pub(super) async fn handle_room_update(
        self: &Arc<Self>,
        data: recv::room::Update,
        initial: bool,
    ) {
        let result = self.settings.check_room_update(&data);

        if let Some(result) = &result {
            if let Some(room) = self.client.room() {
                for output in &result.outputs {
                    room.chat(&format!(
                        "{} {}",
                        match output.level {
                            ConstraintLevel::Error => FAILURE,
                            ConstraintLevel::Warning => WARNING,
                            ConstraintLevel::Info | ConstraintLevel::Change => "",
                        },
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
                    && !result.outputs.is_empty()
                    && let Some(room) = self.client.room()
                {
                    room.chat(&format!(
                        "paste the following command to fix:\n\n/set {}",
                        result
                            .outputs
                            .iter()
                            .map(|x| x.fix.clone())
                            .collect::<Vec<_>>()
                            .join(";")
                    ))
                    .await
                    .unwrap();
                }

                return;
            }
        }
        let attempt = self.state.read().await.enabled.attempt;
        if result
            .as_ref()
            .is_none_or(|r| r.level != ConstraintLevel::Error)
            && attempt
        {
            if let Some(mut room) = self.client.room() {
                room.switch(Bracket::Player).await.ok();
            }
            self.state.write().await.enabled.value = true;
        }
    }
}
