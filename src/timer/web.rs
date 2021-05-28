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
