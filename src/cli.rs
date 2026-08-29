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
            Self::Pin => "usage: git pin [path] | --help | --list | --prune",
            Self::Unpin => "usage: git unpin [path|name]",
        }
    }
}

/// A parsed command invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Invocation {
    Pin(Option<OsString>),
    Help,
    List,
    Prune,
    Unpin(Option<OsString>),
}

/// Complete help for the `git pin` external command.
pub const PIN_HELP: &str = "Git Pin creates and maintains repository launchers for Visual Studio Code.\n\nUSAGE:\n    git pin [path]\n    git pin --list\n    git pin --prune\n    git pin --help\n\nMODES:\n    [path]    Pin the repository containing path (default: current directory)\n    --list    List every Git Pin managed repository launcher and its status\n    --prune   Remove managed launchers whose repository root is no longer valid\n    --help, -h\n              Print this help without changing any launcher\n\nRun `git unpin [path|name]` to remove one managed launcher.\n";

/// Parses zero or one positional argument without interpreting paths as text.
pub fn parse<I>(operation: Operation, arguments: I) -> Result<Invocation, AppError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut arguments = arguments.into_iter();
    let argument = arguments.next();

    if arguments.next().is_some() {
        return Err(AppError::usage(operation.usage()));
    }

    match operation {
        Operation::Pin => match argument.as_deref() {
            None => Ok(Invocation::Pin(None)),
            Some(value) if value == OsStr::new("--help") || value == OsStr::new("-h") => {
                Ok(Invocation::Help)
            }
            Some(value) if value == OsStr::new("--list") => Ok(Invocation::List),
            Some(value) if value == OsStr::new("--prune") => Ok(Invocation::Prune),
            Some(value) if is_option(value) => Err(AppError::usage(operation.usage())),
            Some(_) => Ok(Invocation::Pin(argument)),
        },
        Operation::Unpin if argument.as_deref().is_some_and(is_option) => {
            Err(AppError::usage(operation.usage()))
        }
        Operation::Unpin => Ok(Invocation::Unpin(argument)),
    }
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
    fn accepts_every_valid_pin_invocation() {
        assert_eq!(parse(Operation::Pin, args(&[])).unwrap(), Invocation::Pin(None));
        assert_eq!(
            parse(Operation::Pin, args(&["repository path"])).unwrap(),
            Invocation::Pin(Some(OsString::from("repository path")))
        );
        for (argument, expected) in [
            ("--help", Invocation::Help),
            ("-h", Invocation::Help),
            ("--list", Invocation::List),
            ("--prune", Invocation::Prune),
        ] {
            assert_eq!(parse(Operation::Pin, args(&[argument])).unwrap(), expected);
        }
    }

    #[test]
    fn preserves_the_valid_unpin_argument_matrix() {
        assert_eq!(
            parse(Operation::Unpin, args(&[])).unwrap(),
            Invocation::Unpin(None)
        );
        assert_eq!(
            parse(Operation::Unpin, args(&["repository path"])).unwrap(),
            Invocation::Unpin(Some(OsString::from("repository path")))
        );
    }

    #[test]
    fn rejects_unknown_options_combined_modes_and_extra_arguments() {
        for invalid in [
            args(&["one", "two"]),
            args(&["--unknown"]),
            args(&["--list", "repository"]),
            args(&["repository", "--prune"]),
            args(&["--help", "--list"]),
            args(&["-h", "--prune"]),
        ] {
            let error = parse(Operation::Pin, invalid).expect_err("pin arguments must be rejected");
            assert_eq!(error.exit_code(), ExitCode::Usage);
            assert_eq!(error.to_string(), Operation::Pin.usage());
        }

        for invalid in [args(&["one", "two"]), args(&["--list"]), args(&["-h"])] {
            let error =
                parse(Operation::Unpin, invalid).expect_err("unpin arguments must be rejected");
            assert_eq!(error.exit_code(), ExitCode::Usage);
            assert_eq!(error.to_string(), Operation::Unpin.usage());
        }
    }
}
