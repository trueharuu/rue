use rue_core::{
    game::Game,
    placement::Move,
    piece::Piece,
    rotation::Rotation,
    spin::Spin,
};
use rue_eval::weights::Weights;
use rue_nav::{buffer::Moves, movegen};
use rustc_hash::FxHashMap;

use rayon::prelude::*;

use crate::config::{SearchConfig, SearchExpansionContext, SearchNode};

/// Generate all root-level nodes by applying each legal placement.
pub(crate) fn expand_root<const N: usize, W: Weights + Sync>(
    game: &Game<N>,
    ctx: &mut SearchExpansionContext<'_, W, N>,
) -> Vec<SearchNode<N>> {
    if game.queue.is_empty() {
        return Vec::new();
    }

    let moves_a =
        movegen::generate(&game.board, game.ruleset, game.queue[0], game.board.max_y(), 0);

    let moves_b = if game.hold.is_some() || game.queue.len() >= 2 {
        let second = game.hold.unwrap_or_else(|| game.queue[1]);
        if second == game.queue[0] {
            Moves::empty(second)
        } else {
            movegen::generate(&game.board, game.ruleset, second, game.board.max_y(), 0)
        }
    } else {
        Moves::empty(game.queue[0])
    };

    let config = ctx.config;
    let weights = ctx.weights;
    let remaining_depth = ctx.remaining_depth;
    let queue_first = game.queue[0];

    let all_moves: Vec<_> = moves_a.iter().chain(moves_b.iter()).collect();

    all_moves
        .par_iter()
        .map(|&mv| {
            let mut child = game.clone();
            let attack_ctx = child.tick(mv);

            let path = vec![mv];
            let mut local_tt = None;
            let board_eval = evaluate_with_tt(
                &child,
                weights,
                remaining_depth,
                &mut local_tt,
                Some(&path),
            );
            let attack_val = f64::from(attack_ctx.attack_sent);
            let chain_val = shape_chain_value(attack_ctx.combo_after);
            let score = assemble_composite(board_eval, attack_val, chain_val, config);

            let hold_used = mv.piece() != queue_first;

            SearchNode {
                game: child,
                score,
                root_move: mv,
                root_hold_used: hold_used,
                path,
                attack_score: attack_val,
                chain_score: chain_val,
                board_score: board_eval,
                path_attack: attack_val,
                path_chain: chain_val,
            }
        })
        .collect()
}

/// Expand a single node into all its children.
pub(crate) fn expand_node<const N: usize, W: Weights + Sync>(
    parent: &SearchNode<N>,
    ctx: &mut SearchExpansionContext<'_, W, N>,
    out: &mut Vec<SearchNode<N>>,
) {
    if parent.game.queue.is_empty() {
        return;
    }

    let current_piece = parent.game.queue[0];
    let moves_a = movegen::generate(
        &parent.game.board,
        parent.game.ruleset,
        current_piece,
        parent.game.board.max_y(),
        0,
    );

    let moves_b = if parent.game.hold.is_some() || parent.game.queue.len() >= 2 {
        let second = parent.game.hold.unwrap_or_else(|| parent.game.queue[1]);
        if second == current_piece {
            Moves::empty(second)
        } else {
            movegen::generate(
                &parent.game.board,
                parent.game.ruleset,
                second,
                parent.game.board.max_y(),
                0,
            )
        }
    } else {
        Moves::empty(current_piece)
    };

    let depth_factor = (parent.path.len() as f64 + 1.0)
        .sqrt()
        .min(ctx.config.max_depth_factor);

    let parent_hold_used = parent.root_hold_used;

    for mv in moves_a.iter().chain(moves_b.iter()) {
        let mut child = parent.game.clone();
        let attack_ctx = child.tick(mv);

        let mut path = parent.path.clone();
        path.push(mv);

        let board_eval =
            evaluate_with_tt(&child, ctx.weights, ctx.remaining_depth, ctx.tt, Some(&path));
        let attack_val = f64::from(attack_ctx.attack_sent);
        let chain_val = shape_chain_value(attack_ctx.combo_after);

        let cum_attack = parent.path_attack + attack_val;
        let cum_chain = parent.path_chain + chain_val;

        let score = assemble_composite(
            board_eval,
            cum_attack / depth_factor,
            cum_chain / depth_factor,
            ctx.config,
        );

        out.push(SearchNode {
            game: child,
            score,
            root_move: parent.root_move,
            root_hold_used: parent_hold_used,
            path,
            attack_score: attack_val,
            chain_score: chain_val,
            board_score: board_eval,
            path_attack: cum_attack,
            path_chain: cum_chain,
        });
    }
}

/// Hash a game state for transposition detection.
pub(crate) fn game_hash<const N: usize>(game: &Game<N>) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for col in game.board.as_cols() {
        h = h.wrapping_mul(0x0100_0000_01B3).wrapping_add(col);
    }
    if let Some(p) = game.hold {
        h = h
            .wrapping_mul(0x0100_0000_01B3)
            .wrapping_add(u64::from(p as u8) + 1);
    }
    if let Some(&p) = game.queue.first() {
        h = h
            .wrapping_mul(0x0100_0000_01B3)
            .wrapping_add(u64::from(p as u8) + 1);
    }
    if let Some(b) = game.b2b_count {
        h = h
            .wrapping_mul(0x0100_0000_01B3)
            .wrapping_add(u64::from(b) + 1);
    }
    if let Some(c) = game.combo_count {
        h = h
            .wrapping_mul(0x0100_0000_01B3)
            .wrapping_add(u64::from(c) + 1);
    }

    // Hash the garbage queue by its segments, ignoring any trailing zeroes.
    let mut garbage_hash: u64 = 0xCBF2_9CE4_8422_2325;
    for &segment in &game.garbage_queue.segments {
        garbage_hash = garbage_hash
            .wrapping_mul(0x0100_0000_01B3)
            .wrapping_add(u64::from(segment));
    }
    h
}

/// Shape the chain maintenance term from combo count.
fn shape_chain_value(combo: u32) -> f64 {
    if combo == 0 {
        0.0
    } else {
        (1.0 + f64::from(combo) * 0.25).ln()
    }
}

/// Assemble the composite score from board evaluation, attack, and chain terms.
fn assemble_composite(board: f64, attack: f64, chain: f64, config: &SearchConfig) -> f64 {
    config.board_weight * board + config.attack_weight * attack + config.chain_weight * chain
}

/// Build a dummy `AttackContext` for static board evaluation.
/// The `Weights::evaluate` trait requires an `AttackContext`, but for board-only
/// scoring (used in TT caching and static eval) we pass zeroed values so only
/// the structural features (height, holes, bumpiness) contribute.
fn dummy_attack_ctx<const N: usize>(game: &Game<N>) -> rue_core::game::attack::AttackContext {
    let piece = game.queue.first().copied().unwrap_or(Piece::T);
    rue_core::game::attack::AttackContext {
        clear_type: rue_core::game::attack::Clear::None,
        spin_type: Spin::None,
        lines_cleared: 0,
        attack_sent: 0.0,
        b2b_before: game.b2b_count.unwrap_or(0) as u8,
        b2b_after: game.b2b_count.unwrap_or(0) as u8,
        combo_before: game.combo_count.unwrap_or(0),
        combo_after: game.combo_count.unwrap_or(0),
        is_surge_release: false,
        is_garbage_clear: false,
        is_perfect_clear: false,
        piece,
        placement: Move::new(piece, Rotation::North, 0, 0, Spin::None),
    }
}

/// Evaluate a board with optional transposition table caching.
///
/// When `path` is `Some`, uses [`Weights::evaluate_with_path`] which includes
/// piece-history awareness (relevant for the `Deep` model).  The TT still keys
/// on board hash only — different paths reaching the same position share the
/// cached value, which is a reasonable approximation since history effects are
/// secondary to board structure.
///
/// When `path` is `None`, falls back to the board-only [`Weights::evaluate`].
fn evaluate_with_tt<const N: usize, W: Weights>(
    game: &Game<N>,
    weights: &W,
    remaining_depth: usize,
    tt: &mut Option<FxHashMap<u64, (u8, f64)>>,
    path: Option<&[Move]>,
) -> f64 {
    if let Some(table) = tt.as_mut() {
        let depth = remaining_depth.min(u8::MAX as usize) as u8;
        let hash = game_hash(game);

        if let Some(&(cached_depth, score)) = table.get(&hash)
            && cached_depth >= depth
        {
            return score;
        }

        let ctx = dummy_attack_ctx(game);
        let score = match path {
            Some(p) => weights.evaluate_with_path(game, &ctx, p),
            None => weights.evaluate(game, &ctx),
        };
        table.insert(hash, (depth, score));
        return score;
    }

    let ctx = dummy_attack_ctx(game);
    match path {
        Some(p) => weights.evaluate_with_path(game, &ctx, p),
        None => weights.evaluate(game, &ctx),
    }
}
