#[cfg(target_arch = "wasm32")]
pub fn warn(msg: &str) {
    web_sys::console::warn_1(&msg.into());
}

#[cfg(not(target_arch = "wasm32"))]
pub fn warn(msg: &str) {
    eprintln!("[warn] {}", msg);
}

#[cfg(target_arch = "wasm32")]
pub fn error(msg: &str) {
    web_sys::console::error_1(&msg.into());
}

#[cfg(not(target_arch = "wasm32"))]
pub fn error(msg: &str) {
    eprintln!("[error] {}", msg);
}
