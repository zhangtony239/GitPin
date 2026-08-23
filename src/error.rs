//! User-facing application errors and process exit behavior.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// A diagnostic error suitable for printing to standard error.
#[derive(Debug)]
pub struct AppError {
    message: String,
}

impl AppError {
    pub(crate) fn not_implemented(operation: &str) -> Self {
        Self {
            message: format!("{operation} is not implemented yet"),
        }
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
            1
        }
    }
}

