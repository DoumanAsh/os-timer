use core::time;

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

unsafe impl Send for Timer {}
unsafe impl Sync for Timer {}

impl Timer {
    #[inline(always)]
    ///Creates new schedule
    pub const fn schedule(&self) -> Schedule<'_> {
        Schedule {
            timer: self,
            timeout: time::Duration::from_secs(0),
            interval: time::Duration::from_secs(0),
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    #[inline(always)]
    ///Schedules timer to alarm once after `timeout` passes.
    ///
    ///Note that if timer has been scheduled before, but hasn't expire yet, it shall be cancelled.
    ///To prevent that user must `cancel` timer first.
    ///
    ///Returns `true` if successfully set, otherwise on error returns `false`
    pub fn schedule_once(&self, timeout: time::Duration) -> bool {
        self.schedule_interval(timeout, time::Duration::from_secs(0))
    }
}

///Timer's schedule
pub struct Schedule<'a> {
    timer: &'a Timer,
    timeout: time::Duration,
    interval: time::Duration,
}

impl<'a> Schedule<'a> {
    #[inline(always)]
    ///Sets initial `timeout` to fire timer.
    pub const fn initial(mut self, timeout: time::Duration) -> Self {
        self.timeout = timeout;
        self
    }

    #[inline(always)]
    ///Sets `timeout` interval to run periodically after `initial` has been fired
    ///
    ///Note that if `timeout` is zero behavior depends on underlying OS API.
    ///But most often than note it will fire immediately.
    pub const fn interval(mut self, timeout: time::Duration) -> Self {
        self.interval = timeout;
        self
    }

    #[inline(always)]
    ///Schedules timer execution, using provided settings.
    ///
    ///Returns `true` if successfully set, otherwise on error returns `false`
    pub fn schedule(&self) -> bool {
        self.timer.schedule_interval(self.timeout, self.interval)
    }
}
