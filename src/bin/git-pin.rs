use git_pin::cli::Operation;

fn main() {
    let result = git_pin::cli::parse(Operation::Pin, std::env::args_os().skip(1))
        .and_then(git_pin::app::run);
    let code = git_pin::error::report(result);
    std::process::exit(code);
}
