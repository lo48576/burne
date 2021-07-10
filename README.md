# burne

[![Build Status](https://gitlab.com/lo48576/burne/badges/develop/pipeline.svg)](https://gitlab.com/lo48576/burne/pipelines/)
![Minimum supported rustc version: 1.53](https://img.shields.io/badge/rustc-1.53+-lightgray.svg)

BUlk ReName by Editor.

## Usage
For usage, see `burne --help`.

```
$ burne --help
burne
Renames child files in a directory using editor

USAGE:
    burne [FLAGS] [OPTIONS] [source-dir]

ARGS:
    <source-dir>
            Source directory that contains files to rename [default: .]

FLAGS:
    -n, --dry-run
            Instead of running rename, just prints filenames before and after the rename

    -h, --help
            Prints help information

    -z, --null-data
            Separates the lines by NUL characters

    -p, --parents
            *UNIMPLEMENTED*: Makes parent directories for destination paths as needed.

            Not yet implemented.

    -V, --version
            Prints version information


OPTIONS:
    -e, --escape <escape>
            Escape method [default: none] [possible values: none, percent, percent-encoding]
```

### Escape method

Sometimes you need to handle special characters such as `\n` and/or invalid UTF-8 sequences.
This is when `--escape` shines.

`--escape=none` (default) does not do any escape.
This would be most intuitive for editing.
However, burne fails if the paths before/after rename includes special characters
such as the line separator and filenames cannot be separated unambiguously.

`--escape=percent` applies percent encoding to lines in the file to be edited.
For example, when you do `touch hello$'\n'world` and run burne with `--escape percent`,
then you will see `hello%0Aworld` in your editor.
You can use percent-encoded sequences (such as `%20` for a whitespace)
when you write new filenames.

`--escape=percent-ascii` is similar to `--escape=percent`, but this escapes more characters:
not only ASCII control characters, but also all non-ASCII characters!
If your editor cannot handle arbitrary UTF-8 strings, you can use this method to read and write
only ASCII characters.

### Null data

Usually, line feed (`\n`) character is used as a line separator in the file you edit.
However, sometimes source/destination files can contain `\n` character, and you
will want to handle such special characters unambiguously in your editor without
escape.

`--null-data` let burne use `\0` (NUL character) as a line separator.
Paths cannot contain `\0`, so this makes separation of unescaped filenames unambiguous.

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE.txt](LICENSE-APACHE.txt) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT.txt](LICENSE-MIT.txt) or
  <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
