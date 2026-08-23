//! Minimal parsing for the two Git external commands.

use std::ffi::{OsStr, OsString};

use crate::error::AppError;

/// Stable operations exposed by the application layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operation {
    Pin,
    Unpin,
}

impl Operation {
    /// The executable name and usage accepted by this operation.
    pub const fn usage(self) -> &'static str {
        match self {
            Self::Pin => "usage: git pin [path]",
            Self::Unpin => "usage: git unpin [path|name]",
        }
    }
}

/// A parsed command invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Invocation {
    pub operation: Operation,
    pub argument: Option<OsString>,
}

/// Parses zero or one positional argument without interpreting paths as text.
pub fn parse<I>(operation: Operation, arguments: I) -> Result<Invocation, AppError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut arguments = arguments.into_iter();
    let argument = arguments.next();

    if argument.as_deref().is_some_and(is_option) || arguments.next().is_some() {
        return Err(AppError::usage(operation.usage()));
    }

    Ok(Invocation {
        operation,
        argument,
    })
}

fn is_option(argument: &OsStr) -> bool {
    argument.to_string_lossy().starts_with('-')
}

#[cfg(test)]
mod tests {
    use super::{parse, Invocation, Operation};
    use crate::error::ExitCode;
    use std::ffi::OsString;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn accepts_the_valid_argument_matrix_for_both_commands() {
        for operation in [Operation::Pin, Operation::Unpin] {
            assert_eq!(
                parse(operation, args(&[])).expect("zero arguments are valid"),
                Invocation {
                    operation,
                    argument: None,
                }
            );
            assert_eq!(
                parse(operation, args(&["repository path"]))
                    .expect("one positional argument is valid"),
                Invocation {
                    operation,
                    argument: Some(OsString::from("repository path")),
                }
            );
        }
    }

    #[test]
    fn rejects_options_and_extra_arguments_for_both_commands() {
        for operation in [Operation::Pin, Operation::Unpin] {
            for invalid in [
                args(&["one", "two"]),
                args(&["--name"]),
                args(&["--list"]),
                args(&["--prune"]),
                args(&["--all"]),
                args(&["-h"]),
            ] {
                let error = parse(operation, invalid).expect_err("arguments must be rejected");
                assert_eq!(error.exit_code(), ExitCode::Usage);
                assert_eq!(error.to_string(), operation.usage());
            }
        }
    }
}
