//! Renamer.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io::{self, BufRead, Write};
#[cfg(unix)]
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail};

/// Characters to be escaped by percent encoding.
const PERCENT_ENCODE_ESCAPE_SET: &percent_encoding::AsciiSet =
    &percent_encoding::CONTROLS.add(b' ').add(b'\n');

/// Escape method.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Escape {
    /// No escape.
    ///
    /// Cannot rename to filenames with special characters, and fails if the
    /// source filenames contains special characters.
    None,
    /// Percent encoding.
    PercentEncoding,
}

impl Escape {
    /// Writes the escaped path.
    ///
    /// # Failures
    ///
    /// Fails if the given path contains a sequence that cannot be escaped
    /// safely by this escape method.
    #[cfg(unix)]
    fn escape<W: Write>(
        self,
        mut writer: W,
        path: &Path,
        line_sep: LineSeparator,
    ) -> anyhow::Result<()> {
        match self {
            Self::None => match path.to_str() {
                Some(s) => {
                    if s.contains(line_sep.to_char()) {
                        return Err(anyhow!(
                            "the path {:?} cannot be escaped with the escape method `none`: \
                             line separator {:?} found",
                            s,
                            line_sep
                        ));
                    }
                    write!(writer, "{}", s)?;

                    Ok(())
                }
                None => Err(anyhow!(
                    "the path {:?} cannot be escaped with the escape method `none`: \
                     invalid UTF-8 sequence",
                    path
                )),
            },
            Self::PercentEncoding => {
                let encoded = percent_encoding::percent_encode(
                    path.as_os_str().as_bytes(),
                    PERCENT_ENCODE_ESCAPE_SET,
                );
                assert!(
                    encoded
                        .clone()
                        .flat_map(|s| s.bytes())
                        .all(|b| b != line_sep.to_byte()),
                    "escaped path string should not contain line separators"
                );
                write!(writer, "{}", encoded)?;

                Ok(())
            }
        }
    }

    /*
    /// Unescapes the path by the escape method.
    ///
    /// # Failures
    ///
    /// Fails if the given path contains a sequence that cannot be escaped
    /// safely by this escape method.
    fn unescape(self, s: &str, _line_sep: LineSeparator) -> anyhow::Result<Cow<'_, Path>> {
        match self {
            Self::None => Ok(Cow::Borrowed(Path::new(s))),
            Self::PercentEncoding => Ok(Cow::Owned(PathBuf::from(OsString::from_vec(
                percent_encoding::percent_decode(s.as_bytes()).collect(),
            )))),
        }
    }
    */

    /// Unescapes the path by the escape method.
    ///
    /// # Failures
    ///
    /// Fails if the given path contains a sequence that cannot be escaped
    /// safely by this escape method.
    fn unescape_read_line<R: BufRead>(
        self,
        line_sep: LineSeparator,
        reader: &mut R,
    ) -> anyhow::Result<Option<OsString>> {
        // Use `BufRead::has_data_left` once it is stabilized.
        // See <https://github.com/rust-lang/rust/issues/86423>.
        if reader.fill_buf()?.is_empty() {
            return Ok(None);
        }

        let mut bytes = Vec::new();
        reader.read_until(line_sep.to_byte(), &mut bytes)?;
        if bytes.last() == Some(&line_sep.to_byte()) {
            bytes.pop();
        }
        match self {
            Self::None => Ok(Some(OsString::from_vec(bytes))),
            Self::PercentEncoding => Ok(Some(OsString::from_vec(
                percent_encoding::percent_decode(&bytes).collect(),
            ))),
        }
    }
}

impl Escape {
    /// Creates an escape method value from the given string.
    ///
    /// This is intended for use with CLI parser.
    pub(crate) fn try_from_cli_str(s: &str) -> anyhow::Result<Self> {
        match s {
            "none" => Ok(Self::None),
            "percent" | "percent-encoding" => Ok(Self::PercentEncoding),
            s => Err(anyhow!("unknown escape method {:?}", s)),
        }
    }

    /// Returns the possible CLI string representation of the `Escape` variants.
    ///
    /// This is intended for use with CLI parser.
    pub(crate) fn cli_possible_values() -> &'static [&'static str] {
        &["none", "percent", "percent-encoding"]
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

impl LineSeparator {
    /// Returns the line separator character.
    #[inline]
    fn to_char(self) -> char {
        match self {
            Self::LineFeed => '\n',
            Self::Null => '\0',
        }
    }

    /// Returns the line separator character as an ASCII byte.
    #[inline]
    fn to_byte(self) -> u8 {
        match self {
            Self::LineFeed => b'\n',
            Self::Null => b'\0',
        }
    }
}

/// Setup of a bulk rename.
#[derive(Debug, Clone)]
pub(crate) struct RenameSetup {
    /// Source directory.
    source_dir: PathBuf,
    /// Source entries.
    entries: Vec<OsString>,
}

impl RenameSetup {
    /// Creates a new `RenameSetup` for the given directory.
    #[inline]
    pub(crate) fn new<P: Into<PathBuf>>(source_dir: P) -> anyhow::Result<Self> {
        Self::new_impl(source_dir.into())
    }

    /// Creates a new `RenameSetup` for the given directory.
    fn new_impl(source_dir: PathBuf) -> anyhow::Result<Self> {
        // Get source filenames.
        let mut entries = std::fs::read_dir(&source_dir)?
            .map(|entry_res| entry_res.map(|entry| entry.file_name()))
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort();

        Ok(Self {
            source_dir,
            entries,
        })
    }

    /// Writes the entries to a writer.
    pub(crate) fn write<W: Write>(
        &self,
        mut writer: W,
        escape: Escape,
        line_sep: LineSeparator,
    ) -> anyhow::Result<()> {
        for entry in &self.entries {
            escape.escape(&mut writer, Path::new(entry), line_sep)?;
            write!(writer, "{}", line_sep.to_char())?;
        }

        Ok(())
    }

    /// Creates a plan of a bulk rename.
    pub(crate) fn plan<R: BufRead>(
        &self,
        reader: &mut R,
        escape: Escape,
        line_sep: LineSeparator,
    ) -> anyhow::Result<RenamePlan> {
        // A map from destination from source.
        // This is reversed in order to detect duplicate destinations.
        // Sources are guaranteed to be unique since they are filenames in a directory.
        let mut rev_entries: HashMap<OsString, &OsString> = HashMap::new();

        for source in &self.entries {
            let dest = escape
                .unescape_read_line(line_sep, reader)?
                .ok_or_else(|| anyhow!("too few entries in the destination file list"))?;

            if *source == dest {
                log::debug!("source and dest is identical ({:?}). skipping.", source);
                continue;
            }

            log::debug!("new rename entry: source = {:?}, dest = {:?}", source, dest);
            if let Some(another_source) = rev_entries.insert(dest.clone(), source) {
                bail!(
                    "Attempt to rename two files ({:?} and {:?}) to the same name {:?}",
                    another_source,
                    source,
                    dest
                );
            }
        }

        // Key is the last destination, the value is a chain from source to destination.
        let mut seq_chains: HashMap<OsString, Vec<OsString>> = HashMap::new();
        let mut cyclic_chains: Vec<Vec<OsString>> = vec![];

        'collect_chains: loop {
            log::trace!(
                "loop start: seq_chains = {:#?}, cyclic_chains = {:#?}",
                seq_chains,
                cyclic_chains
            );
            // Take a random source-dest pair.
            let dest = match rev_entries.keys().next().cloned() {
                Some(v) => v,
                None => break,
            };
            let source = rev_entries.remove(&dest).expect(
                "should never fail: [consistency] `dest` is a key taken from `rev_entries`",
            );
            log::trace!("entry `{:?} -> {:?}` taken", source, dest);

            // Find a chain to add the pair.
            if let Some(mut chain) = seq_chains.remove(source) {
                debug_assert_eq!(chain.last(), Some(source));
                log::trace!("chain {:?} found", chain);
                chain.push(dest.clone());
                seq_chains.insert(dest, chain);
                continue 'collect_chains;
            }

            // Construct a new chain from dest toward the first source.
            // Note that `chain` here extends from destination to source, but
            // `RenameChain` requires a chain from source to destinaiton.
            // This means that `chain` should be reversed before creating a
            // `RenameChain`.
            log::trace!("creating chain from the destination {:?}", dest);
            let mut chain = vec![dest.clone(), source.clone()];
            let chain = 'trace_chain: loop {
                let chain_last = chain
                    .last()
                    .expect("should never fail: [consistency] `chain` is nonempty");
                let more_source = match rev_entries.remove(chain_last) {
                    Some(v) => v,
                    None => {
                        // Check if there are another joinable chain.
                        log::trace!("finding a joinable chain for {:?} (reversed)", chain);
                        let first_source = chain
                            .last()
                            .expect("should never fail: [consistency] `chain` is nonempty");
                        match seq_chains.remove(first_source) {
                            Some(mut upstream) => {
                                chain.pop();
                                upstream.extend(chain.into_iter().rev());
                                seq_chains.insert(dest, upstream);
                                continue 'collect_chains;
                            }
                            None => {
                                chain.reverse();
                                log::trace!("chain constructed: {:?}", chain);
                                break 'trace_chain chain;
                            }
                        }
                    }
                };
                log::trace!(
                    "more sources found: source={:?} -> dest={:?}",
                    more_source,
                    chain_last
                );
                let chain_first = chain
                    .first()
                    .expect("should never fail: [consistency] `chain` is nonempty");
                if more_source == chain_first {
                    // Loop is detected.
                    chain.reverse();
                    log::trace!("cyclic rename chain found: {:?}", chain);
                    cyclic_chains.push(chain);
                    continue 'collect_chains;
                }
                chain.push(more_source.clone());
            };
            seq_chains.insert(dest, chain);
        }
        log::debug!("chains = {:#?}", seq_chains);
        log::debug!("cyclic chains = {:#?}", cyclic_chains);

        // Use `seq_chains.into_values().collect()` once it is stabilized (at Rust 1.54.0).
        // See <https://github.com/rust-lang/rust/issues/75294>.
        Ok(RenamePlan {
            source_dir: self.source_dir.clone(),
            seq_rename_chains: seq_chains.into_iter().map(|(_k, v)| v).collect(),
            cyclic_rename_chains: cyclic_chains,
        })
    }
}

/// Plan of a bulk rename.
#[derive(Debug, Clone)]
pub(crate) struct RenamePlan {
    /// Source directory.
    source_dir: PathBuf,
    /// Sequential (acyclic) rename chains.
    seq_rename_chains: Vec<Vec<OsString>>,
    /// Cyclic (looped) rename chains.
    cyclic_rename_chains: Vec<Vec<OsString>>,
}

impl RenamePlan {
    /// Runs the rename plan.
    pub(crate) fn run(self, renamer: &Renamer) -> io::Result<()> {
        let source_dir: &Path = &self.source_dir;
        for seq_chain in &self.seq_rename_chains {
            self.rename_seq_chain(seq_chain, &renamer)?;
        }
        if !self.cyclic_rename_chains.is_empty() {
            // Use `tempfile::TempDir::into_path()` in order to avoid user files
            // to be removed by accident when I/O errors happened on rename.
            // In other words, all we need here is just creating a temporary
            // directory with unique name, but not automatically deleting
            // temporary directory (on rename failure).
            let tempdir_path = if renamer.is_dry_run() {
                None
            } else {
                let path = tempfile::Builder::new()
                    .prefix(".burne_")
                    .tempdir_in(source_dir)?
                    .into_path();
                Some(path)
            };
            for cyc_chain in &self.cyclic_rename_chains {
                self.rename_cyc_chain(cyc_chain, tempdir_path.as_deref(), &renamer)?;
            }

            if let Some(tempdir_path) = tempdir_path {
                // Remove the temporary directory.
                // Note that the directory must be empty here.
                fs::remove_dir(&tempdir_path)?;
            }
        }

        Ok(())
    }

    /// Renames a file (or directory).
    ///
    /// `rel_src` and `rel_dest` should be relative to `self.soruce_dir`.
    fn rename_single(
        &self,
        rel_src: impl AsRef<Path>,
        rel_dest: impl AsRef<Path>,
        renamer: &Renamer,
    ) -> io::Result<()> {
        self.rename_single_impl(rel_src.as_ref(), rel_dest.as_ref(), renamer)
    }

    /// Renames a file (or directory).
    ///
    /// `rel_src` and `rel_dest` should be relative to `self.soruce_dir`.
    fn rename_single_impl(
        &self,
        rel_src: &Path,
        rel_dest: &Path,
        renamer: &Renamer,
    ) -> io::Result<()> {
        renamer.rename(&self.source_dir, rel_src, rel_dest)
    }

    /// Renames the given sequential chain using the given temporary directar
    fn rename_seq_chain(&self, seq_chain: &[OsString], renamer: &Renamer) -> io::Result<()> {
        log::trace!("sequential chain: {:?}", seq_chain);
        for src_dest in seq_chain.windows(2).rev() {
            let (src, dest) = match src_dest {
                [src, dest] => (src, dest),
                _ => unreachable!(
                    "item type of `slice::windows(2)` iterator should always be 2-element arrays"
                ),
            };
            self.rename_single(src, dest, renamer)?;
        }

        Ok(())
    }

    /// Runs the given cyclic chain using the given temporary directar
    fn rename_cyc_chain(
        &self,
        cyc_chain: &[OsString],
        tempdir_path: Option<&Path>,
        renamer: &Renamer,
    ) -> io::Result<()> {
        assert_eq!(tempdir_path.is_none(), renamer.is_dry_run());
        let tempdir_path = tempdir_path.unwrap_or_else(|| Path::new("{{tempdir}}"));
        log::trace!("cyclic chain: {:?}", cyc_chain);
        let chain_last = cyc_chain
            .last()
            .expect("should never fail: [consistency] chain has two or more elements");

        // Break the chain.
        let temp_moved = tempdir_path.join(chain_last);
        log::trace!("rename: {:?} => {:?}", chain_last, temp_moved);
        self.rename_single(&chain_last, &temp_moved, renamer)?;

        // Process the chain.
        self.rename_seq_chain(cyc_chain, renamer)?;

        // Complete the cycle.
        let chain_first = cyc_chain
            .first()
            .expect("should never fail: [consistency] chain has two or more elements");
        self.rename_single(&temp_moved, &chain_first, renamer)?;

        Ok(())
    }
}

/// Renamer: an implementation to be used on rename.
#[derive(Debug, Clone)]
pub(crate) enum Renamer {
    /// `std::fs`.
    StdFs,
    /// Dry-run.
    DryRun,
}

impl Renamer {
    /// Returns true if this is a dry-run renamer and does not need any temporary directories.
    #[inline]
    fn is_dry_run(&self) -> bool {
        matches!(*self, Self::DryRun)
    }

    /// Renames the file at the given path.
    fn rename(&self, source_dir: &Path, rel_src: &Path, rel_dest: &Path) -> io::Result<()> {
        match *self {
            Self::StdFs => {
                log::trace!("rename: {:?} => {:?}", rel_src, rel_dest);
                fs::rename(source_dir.join(rel_src), source_dir.join(rel_dest))
            }
            Self::DryRun => {
                println!("{:?} => {:?}", rel_src, rel_dest);
                Ok(())
            }
        }
    }
}
