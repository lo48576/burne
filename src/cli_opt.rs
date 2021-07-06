//! CLI options.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
#[cfg(unix)]
use std::path::PathBuf;

use anyhow::{bail, Context as _};
use clap::Clap;

use crate::renamer::{Escape, LineSeparator, RenameSetup, Renamer};

/// Renames child files in a directory using editor.
#[derive(Debug, Clone, Clap)]
pub(crate) struct Opt {
    /// Source directory that contains files to rename.
    #[clap(default_value = ".")]
    pub(crate) source_dir: PathBuf,
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

impl Opt {
    /// Runs the rename procedure.
    pub(crate) fn run(&self) -> anyhow::Result<()> {
        let setup = RenameSetup::new(&self.source_dir)?;
        log::debug!("setup = {:?}", setup);

        let (mut tempfile, temp_path) = tempfile::NamedTempFile::new()
            .context("failed to create a temporary file")?
            .into_parts();
        log::trace!("temporary file path: {}", temp_path.display());

        setup.write(&mut tempfile, self.escape, self.line_sep)?;
        tempfile.sync_all()?;
        drop(tempfile);

        {
            let editor = Self::get_editor()?;
            let mut command = std::process::Command::new(&editor);
            command.arg(&temp_path);
            let status = command.status()?;
            if !status.success() {
                bail!(
                    "the editor exited unsuccessfully: exit_code={:?}",
                    status.code()
                );
            }
        };

        let mut tempfile = io::BufReader::new(fs::File::open(&temp_path)?);

        let plan = setup.plan(&mut tempfile, self.escape, self.line_sep)?;
        log::trace!("plan = {:#?}", plan);

        let renamer = if self.dry_run {
            Renamer::DryRun
        } else {
            Renamer::StdFs
        };
        plan.run(&renamer)?;

        Ok(())
    }

    /// Attempt to get editor command from the environment.
    fn get_editor() -> anyhow::Result<OsString> {
        // See `$VISUAL` environment variable.
        if let Some(visual) = env::var_os("VISUAL") {
            log::trace!(
                "`$VISUAL` environment variable found (value = {:?})",
                visual
            );
            return Ok(visual);
        }
        // See `$EDITOR` environment variable.
        if let Some(editor) = env::var_os("EDITOR") {
            log::trace!(
                "`$EDITOR` environment variable found (value = {:?})",
                editor
            );
            return Ok(editor);
        }

        bail!("failed to get editor");
    }
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
