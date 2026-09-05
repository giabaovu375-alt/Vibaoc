// ============================================================
// VIBAO RUNTIME (Rust/WASM) — runtime/log.rs
// A shared console-log helper used throughout the runtime.
//
// Why this file exists: calling `web_sys::console::warn_1(...)` directly
// only works when building for the `wasm32-unknown-unknown` target
// (with a real JS glue layer behind it). When running plain
// `cargo test` (the build machine's native target), `web_sys::console`
// still compiles (it's just an FFI binding) but panics at runtime since
// there's no JS engine behind it to receive the call. Wrapped through
// these 2 functions below, picking the correct implementation via
// `#[cfg(target_arch = "wasm32")]`, so `cargo test` (native) runs
// normally during development, while a real (wasm) build still prints
// to the browser console like the original JS version.
// ============================================================

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
