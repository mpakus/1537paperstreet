//! Core application logic for 1537paperstreet.

#![warn(missing_docs)]

pub mod agents;
pub mod config;
pub mod dashboard;
pub mod docio;
pub mod error;
pub mod fsops;
pub mod log;
pub mod mermaid_cache;
pub mod paths;
pub mod projects;
pub mod search;
pub mod store;
pub mod themes;
pub mod tree;
pub mod typescript;
pub mod ui_state;
pub mod updates;
pub mod watch;

pub use error::{Error, Result};
