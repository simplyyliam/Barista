//! Platform-specific code. Barista is Windows-only, so everything Win32 sits in `windows/`
//! and the rest of the crate reads like plain Rust (CLAUDE.md rule 7).

pub mod windows;
