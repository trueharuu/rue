//! Beam search engine for Rue.

#![allow(missing_docs)]

pub mod config;
pub mod search;

/// Internal node expansion helpers.
pub mod expand;

pub use config::SearchConfig;
pub use search::{beam_search, Node, SearchResult};
