#[cfg(not(target_os = "macos"))]
compile_error!("git-pin-launcher is an internal macOS-only target");

#[cfg(target_os = "macos")]
fn main() {
    eprintln!("git-pin-launcher is not implemented yet");
    std::process::exit(1);
}
