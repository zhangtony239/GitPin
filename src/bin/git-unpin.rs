use git_pin::cli::Command;

fn main() {
    let code = git_pin::error::report(git_pin::app::run(Command::Unpin));
    std::process::exit(code);
}
