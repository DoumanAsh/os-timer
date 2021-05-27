//! Cross-platform wrapper over OS timers

#![no_std]
#![warn(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

mod timer;
pub use timer::Timer;

unsafe impl Send for Timer {}
unsafe impl Sync for Timer {}
