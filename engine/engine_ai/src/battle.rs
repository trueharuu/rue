use engine_core::piece::Mino;
use engine_nav::game::Game;
use engine_rng::rng::Rng;

use crate::{beam::Beam, model::Model};

pub fn do_battle(p1: &Model, p2: &Model) -> i32 {
    let mut g1 = Game::new_empty();
    let mut g2 = Game::new_empty();

    let mut q1 = vec![];
    let mut q2 = vec![];

    let mut bag = Mino::bag();
    let mut r = Rng::new_unseeded();
    for _ in 0..10 {
        r.shuffle_array(&mut bag);
        q1.extend(bag);
        q2.extend(bag);
    }

    let mut i1 = 0usize;
    let mut i2 = 0usize;
    let mut p1_turn = true;

    const MAX_TURNS: usize = 54_000;
    for _ in 0..MAX_TURNS {
        if q1.len().saturating_sub(i1) <= 7 || q2.len().saturating_sub(i2) <= 7 {
            r.shuffle_array(&mut bag);
            q1.extend(bag);
            q2.extend(bag);
        }

        let survived = if p1_turn {
            take_turn(&mut g1, p1, &mut q1, &mut i1, &mut g2)
        } else {
            take_turn(&mut g2, p2, &mut q2, &mut i2, &mut g1)
        };

        if !survived {
            return if p1_turn { -1 } else { 1 };
        }

        p1_turn = !p1_turn;
    }

    let h1 = g1.board.max_height();
    let h2 = g2.board.max_height();
    if h1 < h2 {
        1
    } else if h2 < h1 {
        -1
    } else {
        let gb1 = g1.total_garbage();
        let gb2 = g2.total_garbage();
        if gb1 < gb2 {
            1
        } else if gb2 < gb1 {
            -1
        } else {
            0
        }
    }
}

fn take_turn(
    game: &mut Game,
    model: &Model,
    queue: &mut Vec<Mino>,
    index: &mut usize,
    opponent: &mut Game,
) -> bool {
    if *index >= queue.len() {
        return false;
    }

    let slice = &queue[*index..(*index + 7).min(queue.len())];
    let next_move = Beam::new(game, model, slice.len(), 2000).search(slice);

    let Some(next_move) = next_move else {
        return false;
    };

    let held_before = game.hold;
    let pi = game.advance(slice[0], &next_move);
    if held_before.is_none() && game.hold.is_some() {
        queue.remove(*index);
    }
    *index += 1;

    if pi.outgoing_attack > 0 {
        opponent
            .incoming_garbage
            .push(pi.outgoing_attack.min(u8::MAX as u16) as u8);
    }

    true
}
