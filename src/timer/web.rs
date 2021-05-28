use core::time;

#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    fn setTimeout(closure: &wasm_bindgen::closure::Closure<dyn FnMut()>, time: i32) -> i32;
    fn setInterval(closure: &wasm_bindgen::closure::Closure<dyn FnMut()>, time: i32) -> i32;
    fn clearTimeout(id: i32);
    fn clearInterval(id: i32);
}

///Timer for web wasm target
pub struct Timer {
}

impl Timer {
    #[inline]
    ///Creates new uninitialized instance.
    ///
    ///In order to use it one must call `init`.
    pub const unsafe fn uninit() -> Self {
        Self {
        }
    }

    #[inline(always)]
    ///Returns whether timer is initialized
    pub fn is_init(&self) -> bool {
        //!self.inner.load(Ordering::Acquire).is_null()
        true
    }

    ///Schedules timer to alarm periodically with `interval` with initial alarm of `timeout`.
    ///
    ///Note that if timer has been scheduled before, but hasn't expire yet, it shall be cancelled.
    ///To prevent that user must `cancel` timer first.
    ///
    ///# Note
    ///
    ///- `interval` is truncated by `u32::max_value()`
    ///
    ///Returns `true` if successfully set, otherwise on error returns `false`
    pub fn schedule_interval(&self, timeout: time::Duration, interval: time::Duration) -> bool {
        true
    }
}
