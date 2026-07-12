//! SQLite persistence: token ledger and pet/economy snapshot state.
//!
//! See `docs/architecture.md` §Key Dependencies (`rusqlite`). [`Ledger`]
//! (task 0004) is the first real piece - the token event ledger. Task 0006
//! adds a compact economy-state snapshot so pending Food is not lost on app
//! restart. A real migration framework remains follow-on work; bare
//! `CREATE TABLE IF NOT EXISTS` schemas are enough for the current tables.
#![allow(dead_code, unused_imports)]

mod game_state;
mod ledger;

pub use game_state::GameStateStore;
pub use ledger::Ledger;
