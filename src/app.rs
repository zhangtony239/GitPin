//! Application orchestration shared by both command-line entry points.

use crate::cli::Command;
use crate::error::AppError;

/// Runs one of the public commands.
pub fn run(command: Command) -> Result<(), AppError> {
    match command {
        Command::Pin => Err(AppError::not_implemented("git pin")),
        Command::Unpin => Err(AppError::not_implemented("git unpin")),
    }
}
