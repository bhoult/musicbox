// libSDL2 is the only library linked besides libc: the sound device, and the
// window, its renderer and its events when the window is open.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=SDLLIB");
    let sdl = std::env::var("SDLLIB").unwrap_or_else(|_| "/usr/lib/x86_64-linux-gnu/libSDL2-2.0.so.0".into());
    println!("cargo:rustc-link-arg={sdl}");
}
