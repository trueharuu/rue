#![allow(missing_docs)]

use rue_core::board::Board;
use rue_core::piece::Piece;
use rue_core::render;
use rue_core::rule::DEFAULT;
use rue_core::rule::Rule;
use rue_core::spin::Spins;
use rue_nav::movegen::fast;
use rue_nav::movegen::oracle;

fn main() {
    let mut b = Board::empty();
    b.set(0, 0);
    b.set(0, 1);
    b.set(0, 2);
    b.set(1, 0);
    b.set(3, 0);
    b.set(4, 0);
    b.set(5, 0);
    b.set(6, 0);
    b.set(4, 1);
    b.set(4, 2);
    b.set(3, 2);
    b.set(5, 1);
    b.set(6, 1);
    b.set(6, 2);
    b.set(7, 0);
    b.set(7, 1);
    b.set(8, 0);
    b.set(8, 1);
    b.set(9, 0);
    b.set(9, 1);

    let mvs = fast::movegen::<8, DEFAULT>(&b, Piece::J, 20, 0);
    for mv in mvs
        .iter()
        // .filter(|x| x.spin() != rue_core::spin::Spin::None)
    {
        println!("{}{mv:?}", render::placement(&b, &mv));
    }

    println!("{}", mvs.popcount());
}
