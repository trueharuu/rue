use std::time::Instant;

use engine_ai::{beam::Beam, model::Model};
use engine_core::{board::Board, display::render, piece::Mino};
use engine_nav::{game::Game, keyfinder};
use engine_rng::rng::Rng;

pub fn main() {
    let m = Model::new();
    println!("{}", fitness(&m, 5.0, 1000));
}

pub fn fitness(model: &Model, target_pps: f32, max_pieces: usize) -> f32 {
    let mut g = Game {
        b2b: -1,
        board: Board::new(),
        combo: -1,
        hold: None,
        incoming_garbage: 0,
    };
    let mut queue = vec![];
    let mut r = Rng::new_unseeded();
    let mut bag = Mino::bag();
    for _ in 0..(max_pieces / bag.len()) {
        r.shuffle_array(&mut bag);
        queue.extend(bag);
    }
    let t = Instant::now();

    let mut attack = 0;
    for i in 0..queue.len() - 2 {
        // clean simulator
        if i % 14 == 0 {
            // for _ in 0..1 {
            //     g.board
            //         .add_garbage((rand::random::<u64>() % 10) as usize, 1);
            // }

            g.board
                .add_garbage((rand::random::<u64>() % 10) as usize, 8);
        }
        let start = std::time::Instant::now();
        let slice = &queue[i..(i + 7).min(queue.len())];
        let cpy = slice.to_vec();
        let p = Beam::new(&g, &model, slice.len(), 2000).search(slice);
        if p.is_none() {
            return f32::NEG_INFINITY;
        }
        let p = p.unwrap();
        let h = g.hold;
        render(&g.board, Some(p.clone()));
        let keys = keyfinder::keygen(&g.board, &p, true);
        let pi = g.advance(slice[0], &p);
        attack += pi.outgoing_attack;
        if h.is_none() && g.hold.is_some() {
            queue.remove(0);
        }
        println!(
            "queue=[{}]{}, pieces={}, pieces/second={:.2}({:.2}), attack/piece={:.2}({}) attack/minute={:.2}, input={keys:?}",
            h.map(|x| format!("{x:?}")).unwrap_or_else(String::new),
            cpy.iter()
                .map(|m| format!("{m:?}"))
                .collect::<Vec<_>>()
                .join(""),
            i + 1,
            (i + 1) as f64 / t.elapsed().as_secs_f64(),
            1.0 / start.elapsed().as_secs_f64(),
            attack as f64 / (i + 1) as f64,
            pi.outgoing_attack,
            attack as f64 / t.elapsed().as_secs_f64() * 60.0,
        );

        // if last_elapsed = 0 then we just wait 1000 / pps milliseconds
        let wait_time =
            std::time::Duration::from_secs_f32(1.0 / target_pps).saturating_sub(start.elapsed());
        if wait_time > std::time::Duration::ZERO {
            std::thread::sleep(wait_time);
        }
    }

    attack as f32 / max_pieces as f32
}
