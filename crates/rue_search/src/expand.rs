//! Level expansion for beam search.

use rue_core::game::Game;
use rue_core::placement::Move;
use rue_core::rule::Rule;
use rue_eval::model::Model;
use rue_nav::movegen::fast;

use crate::node::Node;

/// Shared scratch for one level expansion.
pub(crate) struct Ctx<'a, 'm, const N: usize, const RULE: Rule, M: Model> {
    pub model: &'m M,
    pub out: &'a mut Vec<Node<N, RULE>>,
}

/// Expands the root state into level zero. Children carry their own move as
/// `root_move`.
pub(crate) fn expand_root<const N: usize, const RULE: Rule, M: Model>(
    ctx: &mut Ctx<'_, '_, N, RULE, M>,
    game: &Game<N, RULE>,
) {
    let root = Node {
        game: *game,
        root_move: Move::null(),
        score: 0.0,
    };
    emit(ctx, root, true);
}

/// Expands one node into its children. Children inherit `parent.root_move`.
pub(crate) fn expand_node<const N: usize, const RULE: Rule, M: Model>(
    ctx: &mut Ctx<'_, '_, N, RULE, M>,
    parent: Node<N, RULE>,
) {
    emit(ctx, parent, false);
}

/// Emits the three branches of a node: no hold, held swap, and first hold.
///
/// The branch selection happens inside `Game::play`, which consumes the queue
/// and hold based on whether the placement differs from the front of the
/// queue.
#[inline]
fn emit<const N: usize, const RULE: Rule, M: Model>(
    ctx: &mut Ctx<'_, '_, N, RULE, M>,
    parent: Node<N, RULE>,
    first: bool,
) {
    let Some(current) = parent.game.queue.get(0).copied() else {
        return;
    };

    // 1. Place the current piece without holding.
    push_placements(ctx, parent, current, first);

    if let Some(held) = parent.game.hold {
        // 2. Play the held piece. Deduction of hold usage happens in `play`.
        push_placements(ctx, parent, held, first);
    } else if parent.game.queue.len() > 1 {
        // 3. First hold: bring the front of the queue down to place.
        push_placements(ctx, parent, parent.game.queue[1], first);
    }
}

/// Generates placements for `piece` and pushes a child per placement.
#[inline]
fn push_placements<const N: usize, const RULE: Rule, M: Model>(
    ctx: &mut Ctx<'_, '_, N, RULE, M>,
    parent: Node<N, RULE>,
    piece: rue_core::piece::Piece,
    first: bool,
) {
    let y = parent.game.board.height();
    let moves = fast::movegen::<N, RULE>(&parent.game.board, piece, y, 0);

    for mv in &moves {
        let mut child = parent.game;
        let attack = child.play(mv);
        let score = ctx.model.evaluate::<N, RULE>(&child, mv, attack);
        ctx.out.push(Node {
            game: child,
            root_move: if first { mv } else { parent.root_move },
            score,
        });
    }
}