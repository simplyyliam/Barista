//! Script items (v0.9, ADRs 0034 and 0035): a runner built into the bar. Scripts in
//! `%APPDATA%\Barista\scripts` run on a timer and their output goes straight into the registry.
// Removed once the manager uses everything.
#![allow(dead_code)]

pub mod name;
pub mod parse;
pub mod run;
