//! Structured Markdown notes, saved selections and explicit review baselines.

mod notes;
mod sets;
mod types;

pub use notes::{create_note, scan};
pub use sets::{mutate, preview};
pub use types::*;

use crate::Error;

fn invalid(message: impl Into<String>) -> Error {
    Error::Library {
        message: message.into(),
    }
}
