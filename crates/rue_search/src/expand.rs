use std::sync::{Arc, Mutex};

use rue_core::{game::Game};
use rue_eval::weights::Weights;
use rue_nav::{buffer::Moves, movegen};
use rustc_hash::FxHashSet;

use crate::search::Node;

#[allow(clippy::implicit_hasher)]
/// Generate all root-level nodes by applying each legal placement.
pub fn expand_root<const N: usize>(
    game: &Game<N>,
    weights: &impl Weights,
    seen: &Mutex<FxHashSet<u64>>,
) -> Vec<Node<N>> {
    if game.queue.is_empty() {
        return Vec::new();
    }

    let current = game.active();
    let moves_a = movegen::generate(&game.board, game.ruleset, current.0, 20, 0);

    let moves_b = if current.0 == current.1 {
        Moves::empty(current.0)
    } else {
        movegen::generate(&game.board, game.ruleset, current.1, 20, 0)
    };
    let mut nodes = Vec::with_capacity((moves_a.count() + moves_b.count()) as usize);

    for mv in moves_a.iter().chain(moves_b.iter()) {
        let mut child = game.clone();
        let mvv = child.tick(mv);

        if !seen.lock().unwrap().insert(game_hash(&child)) {
            continue;
        }

        let score = weights.evaluate(&child, &mvv);
        nodes.push(Node {
            game: child,
            score,
            root_move: mv,
            path: Arc::new(vec![mv]),
        });
    }

    nodes
}

#[allow(clippy::implicit_hasher)]
/// Expand a single node into all its children.
pub fn expand_node<const N: usize>(
    node: &Node<N>,
    weights: &impl Weights,
    seen: &Mutex<FxHashSet<u64>>,
) -> Vec<Node<N>> {
    if node.game.queue.is_empty() {
        return Vec::new();
    }

    let current = node.game.active();
    let moves_a = movegen::generate(&node.game.board, node.game.ruleset, current.0, 20, 0);

    let moves_b = if current.0 == current.1 {
        Moves::empty(current.0)
    } else {
        movegen::generate(&node.game.board, node.game.ruleset, current.1, 20, 0)
    };
    let mut out = Vec::with_capacity((moves_a.count() + moves_b.count()) as usize);

    for mv in moves_a.iter().chain(moves_b.iter()) {
        let mut child = node.game.clone();
        let mvv = child.tick(mv);

        if !seen.lock().unwrap().insert(game_hash(&child)) {
            continue;
        }

        let score = weights.evaluate(&child, &mvv);
        let mut path = Arc::clone(&node.path);
        Arc::make_mut(&mut path).push(mv);
        out.push(Node {
            game: child,
            score,
            root_move: node.root_move,
            path,
        });
    }

    out
}

/// Hash a game state for transposition detection.
fn game_hash<const N: usize>(game: &Game<N>) -> u64 {
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
    h = h
        .wrapping_mul(0x0100_0000_01B3)
        .wrapping_add(u64::from(game.garbage_row));
    h
}
