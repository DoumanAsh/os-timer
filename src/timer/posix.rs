use core::{ptr, time, mem};
use core::sync::atomic::{AtomicUsize, Ordering};

mod ffi {
    use core::mem;
    #[allow(non_camel_case_types)]
    pub type timer_t = usize;

    pub unsafe extern "C" fn timer_handler(value: libc::sigval) {
        let data = value.sival_ptr;
        if data.is_null() {
            return;
        }

        let cb: fn() -> () = mem::transmute(data);

        (cb)();
    }

    #[repr(C)]
    pub struct itimerspec {
        pub it_interval: libc::timespec,
        pub it_value: libc::timespec,
    }

    extern "C" {
        pub fn timer_settime(timerid: timer_t, flags: libc::c_int, new_value: *const itimerspec, old_value: *mut itimerspec) -> libc::c_int;
        pub fn timer_delete(timerid: timer_t);
    }

    #[link(name = "os-timer-posix-c", lind = "static")]
    extern "C" {
        pub fn posix_timer(clock: libc::c_int, cb: Option<unsafe extern "C" fn(value: libc::sigval)>, data: *mut libc::c_void) -> timer_t;
    }
}

///Posix timer wrapper
pub struct Timer {
    inner: AtomicUsize,
}

impl Timer {
    #[inline]
    ///Creates new uninitialized instance.
    ///
    ///In order to use it one must call `init`.
    pub const unsafe fn uninit() -> Self {
        Self {
            inner: AtomicUsize::new(0),
        }
    }

    #[inline(always)]
    fn get_inner(&self) -> usize {
        let inner = self.inner.load(Ordering::Acquire);
        debug_assert_ne!(inner, 0, "Timer has not been initialized");
        inner
    }

    #[inline(always)]
    ///Returns whether timer is initialized
    pub fn is_init(&self) -> bool {
        self.inner.load(Ordering::Acquire) != 0
    }

    #[must_use]
    ///Performs timer initialization
    ///
    ///`cb` pointer to function to invoke when timer expires.
    ///
    ///Returns whether timer has been initialized successfully or not.
    ///
    ///If timer is already initialized does nothing, returning false.
    pub fn init(&self, cb: fn()) -> bool {
        if self.is_init() {
            return false;
        }

        let handle = unsafe {
            ffi::posix_timer(libc::CLOCK_MONOTONIC, Some(ffi::timer_handler), cb as *mut libc::c_void)
        };

        match self.inner.compare_exchange(0, handle, Ordering::SeqCst, Ordering::Acquire) {
            Ok(_) => handle != 0,
            Err(_) => {
                unsafe {
                    ffi::timer_delete(handle);
                }
                false
            }
        }
    }

    ///Creates new timer, invoking provided `cb` when timer expires.
    ///
    ///On failure, returns `None`
    pub fn new(cb: fn()) -> Option<Self> {
        let handle = unsafe {
            ffi::posix_timer(libc::CLOCK_MONOTONIC, Some(ffi::timer_handler), cb as *mut libc::c_void)
        };

        if handle == 0 {
            return None;
        }

        Some(Self {
            inner: AtomicUsize::new(handle)
        })
    }

    ///Schedules timer to alarm periodically with `interval` with initial alarm of `timeout`.
    ///
    ///Note that if timer has been scheduled before, but hasn't expire yet, it shall be cancelled.
    ///To prevent that user must `cancel` timer first.
    ///
    ///Returns `true` if successfully set, otherwise on error returns `false`
    pub fn schedule_interval(&self, timeout: time::Duration, interval: time::Duration) -> bool {
        let it_value = libc::timespec {
            tv_sec: timeout.as_secs() as libc::time_t,
            #[cfg(not(any(target_os = "openbsd", target_os = "netbsd")))]
            tv_nsec: timeout.subsec_nanos() as libc::suseconds_t,
            #[cfg(any(target_os = "openbsd", target_os = "netbsd"))]
            tv_nsec: timeout.subsec_nanos() as libc::c_long,
        };

        let it_interval = libc::timespec {
            tv_sec: interval.as_secs() as libc::time_t,
            #[cfg(not(any(target_os = "openbsd", target_os = "netbsd")))]
            tv_nsec: interval.subsec_nanos() as libc::suseconds_t,
            #[cfg(any(target_os = "openbsd", target_os = "netbsd"))]
            tv_nsec: interval.subsec_nanos() as libc::c_long,
        };

        let new_value = ffi::itimerspec {
            it_interval,
            it_value,
        };

        unsafe {
            ffi::timer_settime(self.get_inner(), 0, &new_value, ptr::null_mut()) == 0
        }
    }

    ///Cancels ongoing timer, if it was armed.
    pub fn cancel(&self) {
        unsafe {
            ffi::timer_settime(self.get_inner(), 0, mem::MaybeUninit::zeroed().assume_init(), ptr::null_mut());
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let handle = self.inner.load(Ordering::Relaxed);
        if handle != 0 {
            self.cancel();
            unsafe {
                ffi::timer_delete(handle)
            }
        }
    }
}
