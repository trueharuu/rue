//! Beam search engine for Rue.

#![feature(min_adt_const_params)]
#![allow(missing_docs)]
#![allow(clippy::missing_docs_in_private_items)]

pub mod config;
pub mod expand;
pub mod search;

pub use config::SearchConfig;
pub use config::SearchNode;
pub use config::SearchResult;
pub use config::SearchResultFull;
pub use search::beam_search;
pub use search::beam_search_forced;
pub use search::beam_search_with_scores;
