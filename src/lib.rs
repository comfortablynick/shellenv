//! Manage environment across shells
//!
//! Shellenv allows you to keep your shell settings in an expressive,
//! human-readable format (TOML).
//!
//! # Supported shells
//!
//! - Bash
//! - Zsh
//! - Fish
//! - PowerShell
//!
//! # Supported variable types
//!
//! - Environment variables
//! - `$PATH` variable
//!   - Easy appending/prepending
//!   - Deduplication
//! - Aliases
//!   - Generates PowerShell functions for complex aliases
//! - Abbreviations (fish)
//!   - Preferred over aliases which are simple function wrappers in fish

pub mod config;
pub mod shell;
pub mod util;
pub mod var;
