use core::{time, mem, ptr};
use core::sync::atomic::{AtomicPtr, AtomicBool, Ordering};

#[allow(non_camel_case_types)]
mod ffi {
    pub use core::ffi::c_void;
    type uintptr_t = usize;
    type c_long = i64;
    type c_ulong = u64;

    pub type dispatch_object_t = *const c_void;
    pub type dispatch_queue_t = *const c_void;
    pub type dispatch_source_t = *const c_void;
    pub type dispatch_source_type_t = *const c_void;
    pub type dispatch_time_t = u64;

    pub const DISPATCH_TIME_FOREVER: dispatch_time_t = !0;
    //pub const DISPATCH_WALLTIME_NOW: dispatch_time_t = !1;
    pub const QOS_CLASS_DEFAULT: c_long = 0x15;

    extern "C" {
        pub static _dispatch_source_type_timer: c_long;

        pub fn dispatch_get_global_queue(identifier: c_long, flags: c_ulong) -> dispatch_queue_t;
        pub fn dispatch_source_create(type_: dispatch_source_type_t, handle: uintptr_t, mask: c_ulong, queue: dispatch_queue_t) -> dispatch_source_t;
        pub fn dispatch_source_set_timer(source: dispatch_source_t, start: dispatch_time_t, interval: u64, leeway: u64);
        pub fn dispatch_source_set_event_handler_f(source: dispatch_source_t, handler: unsafe extern "C" fn(*mut c_void));
        pub fn dispatch_set_context(object: dispatch_object_t, context: *mut c_void);
        pub fn dispatch_resume(object: dispatch_object_t);
        pub fn dispatch_suspend(object: dispatch_object_t);
        pub fn dispatch_release(object: dispatch_object_t);
        pub fn dispatch_source_cancel(object: dispatch_object_t);
        pub fn dispatch_walltime(when: *const c_void, delta: i64) -> dispatch_time_t;
    }
}

//TODO: Investigate why sometimes it is called multiple times
unsafe extern "C" fn timer_handler(data: *mut ffi::c_void) {
    if data.is_null() {
        return;
    }

    let cb: fn() -> () = mem::transmute(data);

    (cb)();
}

///Windows thread pool timer
pub struct Timer {
    inner: AtomicPtr<ffi::c_void>,
    //Suspension count. Incremented suspend, and decremented on each resume
    suspend: AtomicBool,
}

impl Timer {
    #[inline]
    ///Creates new uninitialized instance.
    ///
    ///In order to use it one must call `init`.
    pub const unsafe fn uninit() -> Self {
        Self {
            inner: AtomicPtr::new(ptr::null_mut()),
            //Note timer is created suspended.
            suspend: AtomicBool::new(true),
        }
    }

    #[inline(always)]
    fn get_inner(&self) -> *mut ffi::c_void {
        let inner = self.inner.load(Ordering::Acquire);
        debug_assert!(!inner.is_null(), "Timer has not been initialized");
        inner
    }

    fn suspend(&self) {
        if let Ok(false) = self.suspend.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
            let handle = self.get_inner();
            unsafe {
                ffi::dispatch_suspend(handle);
            }
        }
    }

    fn resume(&self) {
        if let Ok(true) = self.suspend.compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst) {
            let handle = self.get_inner();
            unsafe {
                ffi::dispatch_resume(handle);
            }
        }
    }

    #[inline(always)]
    ///Returns whether timer is initialized
    pub fn is_init(&self) -> bool {
        !self.inner.load(Ordering::Acquire).is_null()
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
            let queue = ffi::dispatch_get_global_queue(ffi::QOS_CLASS_DEFAULT, 0);
            ffi::dispatch_source_create(&ffi::_dispatch_source_type_timer as *const _ as ffi::dispatch_source_type_t, 0, 0, queue)
        };

        match self.inner.compare_exchange(ptr::null_mut(), handle as _, Ordering::SeqCst, Ordering::Acquire) {
            Ok(_) => match handle.is_null() {
                true => false,
                false => {
                    unsafe {
                        ffi::dispatch_source_set_event_handler_f(handle, timer_handler);
                        ffi::dispatch_set_context(handle, cb as *mut _);
                    }
                    true
                }
            },
            Err(_) => {
                unsafe {
                    ffi::dispatch_release(handle);
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
            let queue = ffi::dispatch_get_global_queue(ffi::QOS_CLASS_DEFAULT, 0);
            ffi::dispatch_source_create(&ffi::_dispatch_source_type_timer as *const _ as ffi::dispatch_source_type_t, 0, 0, queue)
        };

        if handle.is_null() {
            return None;
        }

        unsafe {
            ffi::dispatch_source_set_event_handler_f(handle, timer_handler);
            ffi::dispatch_set_context(handle, cb as *mut _);
        }

        Some(Self {
            inner: AtomicPtr::new(handle as _),
            suspend: AtomicBool::new(true),
        })
    }

    ///Schedules timer to alarm once after `timeout` passes.
    ///
    ///Note that if timer has been scheduled before, but hasn't expire yet, it shall be cancelled.
    ///To prevent that user must `cancel` timer first.
    ///
    ///Note that due to dispatch API limitations, `timeout` is truncated by `i64::max_value()`
    pub fn schedule_once(&self, timeout: time::Duration) {
        let handle = self.get_inner();

        self.suspend();

        unsafe {
            let start = ffi::dispatch_walltime(ptr::null(), timeout.as_nanos() as i64);
            ffi::dispatch_source_set_timer(handle, start, ffi::DISPATCH_TIME_FOREVER, 0);
        }

        self.resume();
    }

    ///Schedules timer to alarm periodically with `interval` with initial alarm of `timeout`.
    ///
    ///Note that if timer has been scheduled before, but hasn't expire yet, it shall be cancelled.
    ///To prevent that user must `cancel` timer first.
    ///
    ///# Note
    ///
    ///- `timeout` is truncated by `i64::max_value()`
    ///- `interval` is truncated by `u64::max_value()`
    pub fn schedule_interval(&self, timeout: time::Duration, interval: time::Duration) {
        let handle = self.get_inner();

        self.suspend();

        unsafe {
            let start = ffi::dispatch_walltime(ptr::null(), timeout.as_nanos() as i64);
            ffi::dispatch_source_set_timer(handle, start, interval.as_nanos() as _, 0);
        }

        self.resume();
    }

    #[inline]
    ///Cancels ongoing timer, if it was armed.
    pub fn cancel(&self) {
        self.suspend()
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let handle = self.inner.load(Ordering::Relaxed);
        if !handle.is_null() {
            unsafe {
                ffi::dispatch_source_cancel(handle);

                //It is error to release while source is suspended
                //So we decrement it
                self.resume();

                ffi::dispatch_release(handle);
            }
        }
    }
}
