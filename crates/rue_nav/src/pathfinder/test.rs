#![allow(clippy::unreadable_literal)]
use crate::pathfinder;
use rue_core::board::Board;
use rue_core::game::ruleset::SEASON_2_HANDLING;
use rue_core::placement::Move;

// failed because couldn't 180 inplace? i think?
// also, reaching something that is canonically equal to the target should be valid
// because they fill the same cells and are equivalent when hard-dropped
#[test]
fn fail_1() {
    let board = Board::from_vector([861810856984383, 0, 0, 0, 0, 0, 0, 0].into());
    let mv = unsafe { Move::from_raw(2734759936) };
    let inputs = pathfinder::get_input::<_, { SEASON_2_HANDLING }>(&board, mv);
    assert!(!inputs.is_empty());
}

// somehow the same exact reason?
#[test]
fn fail_2() {
    let board = Board::from_vector([426241851327, 0, 0, 0, 0, 0, 0, 0].into());
    let mv = unsafe { Move::from_raw(2860556288) };
    let inputs = pathfinder::get_input::<_, { SEASON_2_HANDLING }>(&board, mv);
    assert!(!inputs.is_empty());
}

// same thing again, path leads to a west-facing Z piece in the same spot but we're expecting east
#[test]
fn fail_3() {
    let board = Board::from_vector([1031745281991, 0, 0, 0, 0, 0, 0, 0].into());
    let mv = unsafe { Move::from_raw(3380649984) };
    let inputs = pathfinder::get_input::<_, { SEASON_2_HANDLING }>(&board, mv);
    assert!(!inputs.is_empty());
}

// same thing but with I piece
#[test]
fn fail_4() {
    let board = Board::from_vector([847488152330288639, 768, 0, 0, 0, 0, 0, 0].into());
    let mv = unsafe { Move::from_raw(704749568) };
    let inputs = pathfinder::get_input::<_, { SEASON_2_HANDLING }>(&board, mv);
    assert!(!inputs.is_empty());
}

// new case, immobile non-3-corner T spin mini seems to not work
#[test]
fn fail_5() {
    let board = Board::from_vector([3250470463, 0, 0, 0, 0, 0, 0, 0].into());
    let mv = unsafe { Move::from_raw(41984000) };
    let inputs = pathfinder::get_input::<_, { SEASON_2_HANDLING }>(&board, mv);
    assert!(!inputs.is_empty());
}

// immobile, 3-corner T spin mini pointed up, this probably has to do with spin provenance
#[test]
fn fail_6() {
    let board = Board::from_vector([103184014959, 0, 0, 0, 0, 0, 0, 0].into());
    let mv = unsafe { Move::from_raw(33587200) };
    let inputs = pathfinder::get_input::<_, { SEASON_2_HANDLING }>(&board, mv);
    assert!(!inputs.is_empty());
}

// regular t-spin single.
// can't find `None` placement because it's expecting `Full`
#[test]
fn fail_7() {
    let board = Board::from_vector([66879423, 0, 0, 0, 0, 0, 0, 0].into());
    let mv = unsafe { Move::from_raw(453017600) };
    let inputs = pathfinder::get_input::<_, { SEASON_2_HANDLING }>(&board, mv);
    assert!(!inputs.is_empty());
}

// i believe this requires a 180 to get to. same as before, expecting Mini but we never emit it
#[test]
fn fail_8() {
    let board = Board::from_vector([563941838749631, 0, 0, 0, 0, 0, 0, 0].into());
    let mv = unsafe { Move::from_raw(58761216) };
    let inputs = pathfinder::get_input::<_, { SEASON_2_HANDLING }>(&board, mv);
    assert!(!inputs.is_empty());
}

// this requires non-infinite soft drop. this is just straight up not supported.
// this test is mostly a signal for a movegen "fix".
// sonic drops should be gated under `RULE.inf_sdf`
// #[test]
// fn fail_9_expected() {
//     let board = Board::from_vector([15839586959247, 0, 0, 0, 0, 0, 0, 0].into());
//     let mv = unsafe { Move::from_raw(2197848064) };
//     let inputs = pathfinder::get_input(&board, mv);
//     assert!(inputs.is_empty());
// }
