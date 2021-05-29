//! Cross-platform wrapper over OS timers

#![no_std]
#![warn(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

#[cfg(any(windows, unix))]
mod timer;
#[cfg(any(windows, unix))]
pub use timer::*;
