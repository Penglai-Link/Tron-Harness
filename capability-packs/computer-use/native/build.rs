fn main() {
    // Cargo does not propagate a dependency's rustc-link-arg to the final
    // executable. ScreenCaptureKit's Swift bridge needs the system runtime path.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
}
