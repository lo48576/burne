//! CLI options.

use std::path::PathBuf;

use clap::Clap;

use crate::renamer::{Escape, LineSeparator};

/// Renames child files in a directory using editor.
#[derive(Debug, Clone, Clap)]
pub(crate) struct Opt {
    /// Target directory that contains files to rename.
    #[clap(default_value = ".")]
    pub(crate) target: PathBuf,
    /// Command to edit filenames.
    #[clap(short, long)]
    command: Option<Vec<PathBuf>>,
    /// Escape method.
    #[clap(short, long, parse(try_from_str = Escape::try_from_cli_str), default_value = "none")]
    escape: Escape,
    /// Instead of running rename, just prints filenames before and after the rename.
    #[clap(short = 'n', long)]
    dry_run: bool,
    /// Makes parent directories for destination paths as needed.
    #[clap(short, long)]
    parents: bool,
    /// Separates the lines by NUL characters.
    #[clap(short = 'z', long = "null-data", parse(from_flag = line_separator_from_null_data_flag))]
    line_sep: LineSeparator,
}

/// Creates a `LineSeparator` from a `null-data` flag.
#[inline]
#[must_use]
fn line_separator_from_null_data_flag(null_data: bool) -> LineSeparator {
    if null_data {
        LineSeparator::Null
    } else {
        LineSeparator::LineFeed
    }
}
