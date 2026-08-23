//! Minimal command-line types.

/// Public operations exposed as Git external commands.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Command {
    Pin,
    Unpin,
}
