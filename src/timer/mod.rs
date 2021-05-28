#[cfg(windows)]
mod win32;
#[cfg(windows)]
pub use win32::*;

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use apple::*;

#[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]
mod posix;
#[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]
pub use posix::*;

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::*;
