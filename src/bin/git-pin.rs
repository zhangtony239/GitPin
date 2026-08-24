use git_pin::cli::Operation;

fn main() {
    #[cfg(target_os = "macos")]
    if let Some(result) = git_pin::macos_launcher::run_if_managed() {
        if let Err(error) = result {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
        return;
    }

    let result = git_pin::cli::parse(Operation::Pin, std::env::args_os().skip(1))
        .and_then(git_pin::app::run);
    let code = git_pin::error::report(result);
    std::process::exit(code);
}
