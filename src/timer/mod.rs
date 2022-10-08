use core::{mem,time};

extern crate alloc;
use alloc::boxed::Box;

#[derive(PartialEq, Clone, Copy)]
#[repr(C)]
struct FatPtr {
    ptr: usize,
    vtable: usize,
}

impl FatPtr {
    #[inline(always)]
    const fn null() -> Self {
        Self {
            ptr: 0,
            vtable: 0
        }
    }

    #[inline(always)]
    const fn is_null(&self) -> bool {
        self.ptr == 0 && self.vtable == 0
    }
}

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
            timeout: time::Duration::from_millis(0),
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
        //Settings zero initial timeout makes no sense
        assert!(!(timeout.as_secs() == 0 && timeout.as_nanos() == 0), "Cannot set zero initial timeout");
        self.schedule_interval(timeout, time::Duration::from_secs(0))
    }
}

///Timer's schedule
///
///If initial timeout is not configured, then it is set to `interval` timeout
pub struct Schedule<'a> {
    timer: &'a Timer,
    timeout: time::Duration,
    interval: time::Duration,
}

impl<'a> Schedule<'a> {
    #[inline(always)]
    ///Sets initial `timeout` to fire timer.
    pub const fn initial(mut self, timeout: time::Duration) -> Self {
        //Settings zero initial timeout makes no sense
        assert!(!(timeout.as_secs() == 0 && timeout.as_nanos() == 0), "Cannot set zero initial timeout");
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
        if self.timeout == time::Duration::ZERO {
            self.timer.schedule_interval(self.interval, self.interval)
        } else {
            self.timer.schedule_interval(self.timeout, self.interval)
        }
    }
}

struct BoxFnPtr(pub FatPtr);

impl BoxFnPtr {
    #[inline(always)]
    const fn null() -> Self {
        Self(FatPtr::null())
    }

    #[inline(always)]
    const fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl Drop for BoxFnPtr {
    #[inline(always)]
    fn drop(&mut self) {
        if !self.is_null() {
            unsafe {
                let _ = Box::from_raw(mem::transmute::<_, *mut dyn FnMut()>(self.0));
            }
        }
    }
}
