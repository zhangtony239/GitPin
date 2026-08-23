//! Application orchestration shared by both command-line entry points.

use crate::cli::{Invocation, Operation};
use crate::error::AppError;

/// Runs one of the public commands.
pub fn run(invocation: Invocation) -> Result<(), AppError> {
    match invocation.operation {
        Operation::Pin => Err(AppError::not_implemented("git pin")),
        Operation::Unpin => Err(AppError::not_implemented("git unpin")),
    }
}
