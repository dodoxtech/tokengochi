//! SQLite persistence: token ledger, and (later) pet state/inventory.
//!
//! See `docs/architecture.md` §Key Dependencies (`rusqlite`). [`Ledger`]
//! (task 0004) is the first real piece - the token event ledger. Full pet
//! state/inventory persistence and a real migration framework are follow-on
//! work; a bare `CREATE TABLE IF NOT EXISTS` is enough for the one table
//! that exists so far.
#![allow(dead_code, unused_imports)]

mod ledger;

pub use ledger::Ledger;
