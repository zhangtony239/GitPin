//! User-facing application errors and process exit behavior.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// A diagnostic error suitable for printing to standard error.
#[derive(Debug)]
pub struct AppError {
    message: String,
    exit_code: ExitCode,
}

/// Stable process exit codes exposed by both commands.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExitCode {
    Failure = 1,
    Usage = 2,
}

impl AppError {
    pub(crate) fn not_implemented(operation: &str) -> Self {
        Self {
            message: format!("{operation} is not implemented yet"),
            exit_code: ExitCode::Failure,
        }
    }

    pub(crate) fn usage(usage: &'static str) -> Self {
        Self {
            message: usage.to_owned(),
            exit_code: ExitCode::Usage,
        }
    }

    pub(crate) fn failure(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: ExitCode::Failure,
        }
    }

    pub fn exit_code(&self) -> ExitCode {
        self.exit_code
    }
}
impl Display for AppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for AppError {}

/// Converts a command result into a stable process exit code.
pub fn report(result: Result<(), AppError>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("error: {error}");
            error.exit_code() as i32
        }
    }
}
