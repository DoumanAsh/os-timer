//! Cross-platform wrapper over OS timers
//!
//! # Requirements
//!
//! - Posix timer requires compilation of C shim (i.e. Correct C compiler must be available when
//! compiling for posix target).

#![no_std]
#![warn(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

#[cfg(any(windows, unix))]
mod timer;
#[cfg(any(windows, unix))]
pub use timer::*;
