#![allow(missing_docs)]
#![feature(min_adt_const_params)]

use std::marker::ConstParamTy;
use std::str::FromStr;
use std::time::Instant;

use clap::Parser;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use rue_core::board::Board;
use rue_core::piece::Piece;
use rue_core::rule::Rule;
use rue_core::spin::Spins;
use rue_nav::buffer::Moves;
use rue_nav::movegen::fast;
use rue_nav::movegen::oracle;

#[derive(Debug, clap::Parser)]
pub struct Program {
    queues: Vec<String>,
    #[arg(long)]
    model: Model,
    #[arg(long)]
    mt: bool,
}

#[derive(Debug, clap::ValueEnum, Clone, Copy, PartialEq, Eq, ConstParamTy)]
pub enum Model {
    Oracle,
    Fast,
}

pub const PERFT_RULE: Rule = Rule {
    inf_sdf: false,
    allow_180: true,
    spins: Spins::None,
    das: true,
    spawn_x: 4,
    spawn_y: 21,
};

/// Entry point.
///
/// # Panics
/// Panics when the queue string is invalid (contains non-tetromino characters).
#[inline]
pub fn main() {
    let b = Board::<8>::empty();
    let args = Program::parse();
    for queue in args.queues {
        let pq = parse_queue(&queue);
        let i = Instant::now();
        let total = match args.model {
            Model::Oracle => perft_rec::<8, PERFT_RULE, { Model::Oracle }>(&pq, b, args.mt),
            Model::Fast => perft_rec::<8, PERFT_RULE, { Model::Fast }>(&pq, b, args.mt),
        };
        let elapsed = i.elapsed();

        println!(
            "perft({queue}) = {total} in {elapsed:?} ({} n/s)",
            human(total as f64 / elapsed.as_secs_f64())
        );
    }
}

fn parse_queue(queue: &str) -> Vec<Piece> {
    queue
        .chars()
        .map(|c| Piece::from_str(&c.to_string()).unwrap())
        .collect()
}

#[inline(always)]
fn perft_rec<const N: usize, const RULE: Rule, const MODEL: Model>(
    queue: &[Piece],
    b: Board<N>,
    mt: bool,
) -> u64 {
    if queue.is_empty() {
        return 0;
    }

    if queue.len() == 1 {
        return match queue[0] {
            Piece::I => count::<N, { Piece::I }, RULE, MODEL>(&b),
            Piece::O => count::<N, { Piece::O }, RULE, MODEL>(&b),
            Piece::T => count::<N, { Piece::T }, RULE, MODEL>(&b),
            Piece::J => count::<N, { Piece::J }, RULE, MODEL>(&b),
            Piece::L => count::<N, { Piece::L }, RULE, MODEL>(&b),
            Piece::S => count::<N, { Piece::S }, RULE, MODEL>(&b),
            Piece::Z => count::<N, { Piece::Z }, RULE, MODEL>(&b),
        };
    }

    let nx = match queue[0] {
        Piece::I => movegen::<N, { Piece::I }, RULE, MODEL>(&b),
        Piece::O => movegen::<N, { Piece::O }, RULE, MODEL>(&b),
        Piece::T => movegen::<N, { Piece::T }, RULE, MODEL>(&b),
        Piece::J => movegen::<N, { Piece::J }, RULE, MODEL>(&b),
        Piece::L => movegen::<N, { Piece::L }, RULE, MODEL>(&b),
        Piece::S => movegen::<N, { Piece::S }, RULE, MODEL>(&b),
        Piece::Z => movegen::<N, { Piece::Z }, RULE, MODEL>(&b),
    };

    // if we're at depth <= 3 don't parallelize, the overhead is too high
    if queue.len() <= 3 || !mt {
        return nx
            .into_iter()
            .map(|mv| {
                let mut b2 = b;
                b2.do_move(mv);
                perft_rec::<N, RULE, MODEL>(&queue[1..], b2, mt)
            })
            .sum();
    }

    nx.into_iter()
        .par_bridge()
        .map(|mv| {
            let mut b2 = b;
            b2.do_move(mv);

            perft_rec::<N, RULE, MODEL>(&queue[1..], b2, mt)
        })
        .sum()
}

#[inline(always)]
fn count<const N: usize, const P: Piece, const RULE: Rule, const MODEL: Model>(
    b: &Board<N>,
) -> u64 {
    match const { MODEL } {
        Model::Oracle => oracle::count_locks::<N, P, RULE>(b, 20, 0),
        Model::Fast => fast::count_locks::<N, P, RULE>(b, 20, 0),
    }
}

#[inline(always)]
fn movegen<const N: usize, const P: Piece, const RULE: Rule, const MODEL: Model>(
    b: &Board<N>,
) -> Moves<N> {
    match const { MODEL } {
        Model::Oracle => oracle::generate_inlined::<N, P, RULE>(b, 20, 0).0,
        Model::Fast => fast::generate_inlined::<N, P, RULE, true>(b, 20, 0).0,
    }
}

fn human(n: f64) -> String {
    let mut n = n;
    let mut suffix = "";
    if n >= 1e9 {
        n /= 1e9;
        suffix = "B";
    } else if n >= 1e6 {
        n /= 1e6;
        suffix = "M";
    } else if n >= 1e3 {
        n /= 1e3;
        suffix = "K";
    }
    format!("{n:.2}{suffix}")
}
