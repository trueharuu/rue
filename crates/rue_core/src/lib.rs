//! Core board, piece, and move primitives for Rue's SRS-oriented gameplay engine.
//!
//! The crate exposes bitboard operations, rotation/kick tables, and lightweight
//! placement/render helpers used by higher-level search or analysis crates.

#![feature(portable_simd, min_adt_const_params, const_trait_impl)]
pub mod board;
pub mod data;
pub mod envelope;
pub mod header;
pub mod piece;
pub mod placement;
pub mod render;
pub mod rotation;
pub mod spin;
