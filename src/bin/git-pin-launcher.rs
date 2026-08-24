#[cfg(not(target_os = "macos"))]
compile_error!("git-pin-launcher is an internal macOS-only target");

#[cfg(target_os = "macos")]
fn main() {
    if let Err(error) = git_pin::macos_launcher::run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
