use core::{time, ptr, mem};
use core::sync::atomic::{AtomicPtr, Ordering};

mod ffi {
    pub use core::ffi::c_void;

    type DWORD = u32;
    type BOOL = i32;

    #[repr(C)]
    pub struct FileTime {
        pub low_date_time: DWORD,
        pub high_date_time: DWORD,
    }

    type Callback = Option<unsafe extern "system" fn(cb_inst: *mut c_void, ctx: *mut c_void, timer: *mut c_void)>;

    extern "system" {
        pub fn CloseThreadpoolTimer(ptr: *mut c_void);
        pub fn CreateThreadpoolTimer(cb: Callback, user_data: *mut c_void, env: *mut c_void) -> *mut c_void;
        pub fn SetThreadpoolTimerEx(timer: *mut c_void, pftDueTime: *mut FileTime, msPeriod: DWORD, msWindowLength: DWORD) -> BOOL;
        pub fn WaitForThreadpoolTimerCallbacks(timer: *mut c_void, fCancelPendingCallbacks: BOOL);
    }
}

unsafe extern "system" fn timer_callback(_: *mut ffi::c_void, data: *mut ffi::c_void, _: *mut ffi::c_void) {
    if data.is_null() {
        return;
    }

    let cb: fn() -> () = mem::transmute(data);

    (cb)();
}

///Windows thread pool timer
pub struct Timer {
    inner: AtomicPtr<ffi::c_void>,
}

impl Timer {
    #[inline]
    ///Creates new uninitialized instance.
    ///
    ///In order to use it one must call `init`.
    pub const unsafe fn uninit() -> Self {
        Self {
            inner: AtomicPtr::new(ptr::null_mut())
        }
    }

    #[inline(always)]
    fn get_inner(&self) -> *mut ffi::c_void {
        let inner = self.inner.load(Ordering::Acquire);
        debug_assert!(!inner.is_null(), "Timer has not been initialized");
        inner
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
            ffi::CreateThreadpoolTimer(Some(timer_callback), cb as _, ptr::null_mut())
        };

        match self.inner.compare_exchange(ptr::null_mut(), handle, Ordering::SeqCst, Ordering::Acquire) {
            Ok(_) => !handle.is_null(),
            Err(_) => {
                unsafe {
                    ffi::CloseThreadpoolTimer(handle);
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
            ffi::CreateThreadpoolTimer(Some(timer_callback), cb as _, ptr::null_mut())
        };

        if handle.is_null() {
            return None;
        }

        Some(Self {
            inner: AtomicPtr::new(handle)
        })
    }

    ///Schedules timer to alarm once after `timeout` passes.
    ///
    ///Note that if timer has been scheduled before, but hasn't expire yet, it shall be cancelled.
    ///To prevent that user must `cancel` timer first.
    pub fn schedule_once(&self, timeout: time::Duration) {
        let mut ticks = i64::from(timeout.subsec_nanos() / 100);
        ticks += (timeout.as_secs() * 10_000_000) as i64;
        let ticks = -ticks;

        unsafe {
            let mut time: ffi::FileTime = mem::transmute(ticks);
            ffi::SetThreadpoolTimerEx(self.get_inner(), &mut time, 0, 0);
        }
    }

    ///Schedules timer to alarm periodically with `interval` timeout.
    ///
    ///Note that if timer has been scheduled before, but hasn't expire yet, it shall be cancelled.
    ///To prevent that user must `cancel` timer first.
    ///
    ///Note that due to winapi limitations, `interval` is truncated by `u32::max_value()`
    pub fn schedule_interval(&self, timeout: time::Duration) {
        let mut ticks = i64::from(timeout.subsec_nanos() / 100);
        ticks += (timeout.as_secs() * 10_000_000) as i64;
        let ticks = -ticks;

        let interval = timeout.as_millis() as u32;

        unsafe {
            let mut time: ffi::FileTime = mem::transmute(ticks);
            ffi::SetThreadpoolTimerEx(self.get_inner(), &mut time, interval, 0);
        }
    }

    ///Cancels ongoing timer, if it was armed.
    pub fn cancel(&self) {
        let handle = self.get_inner();
        unsafe {
            ffi::SetThreadpoolTimerEx(handle, ptr::null_mut(), 0, 0);
            ffi::WaitForThreadpoolTimerCallbacks(handle, 1);
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let handle = self.inner.load(Ordering::Relaxed);
        if !handle.is_null() {
            self.cancel();
            unsafe {
                ffi::CloseThreadpoolTimer(handle);
            }
        }
    }
}
