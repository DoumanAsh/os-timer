use core::{ptr, time, mem};
use core::cell::Cell;
use core::sync::atomic::{AtomicUsize, Ordering};
use super::FatPtr;

extern crate alloc;
use alloc::boxed::Box;

mod ffi {
    use core::mem;
    pub use libc::c_void;
    #[allow(non_camel_case_types)]
    pub type timer_t = usize;

    pub type Callback = Option<unsafe extern "C" fn(libc::sigval)>;

    pub unsafe extern "C" fn timer_callback(value: libc::sigval) {
        let cb: fn() -> () = mem::transmute(value.sival_ptr);

        (cb)();
    }

    pub unsafe extern "C" fn timer_callback_unsafe(value: libc::sigval) {
        let cb: unsafe fn() -> () = mem::transmute(value.sival_ptr);

        (cb)();
    }

    pub unsafe extern "C" fn timer_callback_generic<T: FnMut() -> ()>(value: libc::sigval) {
        let cb = &mut *(value.sival_ptr as *mut T);

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

enum CallbackVariant {
    PlainUnsafe(unsafe fn()),
    Plain(fn()),
    Closure(Box<dyn FnMut()>),
}

///Timer's callback abstraction
pub struct Callback {
    variant: CallbackVariant,
    ffi_cb: ffi::Callback,
}

impl Callback {
    ///Creates callback using plain rust function
    pub fn plain(cb: fn()) -> Self {
        Self {
            variant: CallbackVariant::Plain(cb),
            ffi_cb: Some(ffi::timer_callback),
        }
    }

    ///Creates callback using plain unsafe function
    pub fn unsafe_plain(cb: unsafe fn()) -> Self {
        Self {
            variant: CallbackVariant::PlainUnsafe(cb),
            ffi_cb: Some(ffi::timer_callback_unsafe),
        }
    }

    ///Creates callback using closure, storing it on heap.
    pub fn closure<F: 'static + FnMut()>(cb: F) -> Self {
        Self {
            variant: CallbackVariant::Closure(Box::new(cb)),
            ffi_cb: Some(ffi::timer_callback_generic::<F>),
        }
    }
}

///Posix timer wrapper
pub struct Timer {
    inner: AtomicUsize,
    data: Cell<FatPtr>,
}

impl Timer {
    #[inline]
    ///Creates new uninitialized instance.
    ///
    ///In order to use it one must call `init`.
    pub const unsafe fn uninit() -> Self {
        Self {
            inner: AtomicUsize::new(0),
            data: Cell::new(0),
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
    pub fn init(&self, cb: Callback) -> bool {
        if self.is_init() {
            return false;
        }

        let ffi_cb = cb.ffi_cb;
        let ffi_data = match cb.variant {
            CallbackVariant::Plain(cb) => cb as *mut ffi::c_void,
            CallbackVariant::PlainUnsafe(cb) => cb as *mut ffi::c_void,
            CallbackVariant::Closure(ref cb) => cb as *const _ as *mut ffi::c_void,
        };

        let handle = unsafe {
            ffi::posix_timer(libc::CLOCK_MONOTONIC, ffi_cb, ffi_data)
        };

        match self.inner.compare_exchange(0, handle, Ordering::SeqCst, Ordering::Acquire) {
            Ok(_) => match handle {
                0 => false,
                _ => {
                    match cb.variant {
                        CallbackVariant::Closure(cb) => unsafe {
                            //safe because we can never reach here once `handle.is_null() != true`
                            self.data.set(mem::transmute(Box::into_raw(cb)))
                        },
                        _ => (),
                    }
                    true
                },
            },
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
    pub fn new(cb: Callback) -> Option<Self> {
        let ffi_cb = cb.ffi_cb;
        let ffi_data = match cb.variant {
            CallbackVariant::Plain(cb) => cb as *mut ffi::c_void,
            CallbackVariant::PlainUnsafe(cb) => cb as *mut ffi::c_void,
            CallbackVariant::Closure(ref cb) => &*cb as *const _ as *mut ffi::c_void,
        };

        let handle = unsafe {
            ffi::posix_timer(libc::CLOCK_MONOTONIC, ffi_cb, ffi_data)
        };

        if handle == 0 {
            return None;
        }

        let data = match cb.variant {
            CallbackVariant::Closure(cb) => unsafe {
                //safe because we can never reach here once `handle.is_null() != true`
                mem::transmute(Box::into_raw(cb))
            },
            _ => 0,
        };

        Some(Self {
            inner: AtomicUsize::new(handle),
            data: Cell::new(data),
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
            ffi::timer_settime(self.get_inner(), 0, &mem::MaybeUninit::zeroed().assume_init(), ptr::null_mut());
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
