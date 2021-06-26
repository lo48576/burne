//! Renamer.

use anyhow::anyhow;

/// Escape method.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Escape {
    /// No escape.
    ///
    /// Cannot rename to filenames with special characters, and fails if the
    /// source filenames contains special characters.
    None,
}

impl Escape {
    /// Creates an escape method value from the given string.
    ///
    /// This is intended for use with CLI parser.
    pub(crate) fn try_from_cli_str(s: &str) -> anyhow::Result<Self> {
        match s {
            "none" => Ok(Self::None),
            s => Err(anyhow!("unknown escape method {:?}", s)),
        }
    }
}

/// Line separator character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineSeparator {
    /// Line feed character (`\n`).
    LineFeed,
    /// Null character (`\0`).
    Null,
}
