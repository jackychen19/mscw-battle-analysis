fn main() {
    // ScreenCaptureKit's Swift bridge links the Swift runtime, which macOS
    // ships in /usr/lib/swift (dyld shared cache); point the rpath there.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
    tauri_build::build()
}
