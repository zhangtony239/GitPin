//! User-facing application errors and process exit behavior.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};

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
    report_to(result, &mut io::stderr())
}

/// Converts a command result into an exit code and writes diagnostics to `stderr`.
pub fn report_to<W: Write>(result: Result<(), AppError>, stderr: &mut W) -> i32 {
    match result {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            error.exit_code() as i32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{report_to, AppError};

    #[test]
    fn success_is_silent_and_returns_zero() {
        let mut stderr = Vec::new();
        assert_eq!(report_to(Ok(()), &mut stderr), 0);
        assert!(stderr.is_empty());
    }

    #[test]
    fn failures_are_diagnostic_and_use_stable_exit_codes() {
        let mut stderr = Vec::new();
        assert_eq!(
            report_to(
                Err(AppError::failure("could not create launcher")),
                &mut stderr
            ),
            1
        );
        assert_eq!(
            String::from_utf8(stderr).expect("diagnostic must be UTF-8"),
            "error: could not create launcher\n"
        );

        let mut stderr = Vec::new();
        assert_eq!(
            report_to(Err(AppError::usage("usage: git pin [path]")), &mut stderr),
            2
        );
        assert_eq!(
            String::from_utf8(stderr).expect("diagnostic must be UTF-8"),
            "error: usage: git pin [path]\n"
        );
    }
}
