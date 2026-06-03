# jdpackager

Convert a scraper-produced `books.json` into a JDownloader `.crawljob` link container.

Each book becomes one entry. Exactly one download link per book is selected (so each
file can be renamed to its clean Title via the crawljob `filename=` key). All entries
share one `packageName` (the section), so they import into a single JDownloader package.

## Usage

```
jdpackager <input> [--host <rg|nf|dd|name>] [-o <path>] [--no-deploy] [--download-folder <path>]
```

- `<input>` — path to a `books.json` file, or a directory containing one.
- `--host <h>` — preferred host placed at the head of the selection chain. Aliases: `rg`
  (rapidgator), `nf` (nitroflare), `dd` (ddownload). Default chain: `rapidgator -> ddownload -> skip`.
  `--host nf` -> `nitroflare -> rapidgator -> ddownload -> skip`. The preferred host is tried
  first; the rest of the default order follows. nitroflare is never auto-selected without `--host nf`.
- `-o <path>` — override the kept-artifact path. Default: `<exe dir>\.crawljobs\<section>.crawljob`.
  When `-o` is given, the skipped-report sidecar is named after the artifact (`custom.skipped.json`).
- `--no-deploy` — write only the local artifact; don't copy into the Folder Watch folder.
- `--download-folder <path>` — set `downloadFolder=` in every entry (default: JDownloader's default dir).

## Configuration

`jdpackager` copies the generated `.crawljob` into JDownloader's Folder Watch folder so it
auto-imports. Set the folder in `config.toml` next to the executable (see `config.toml.example`).
Defaults to `%LOCALAPPDATA%\JDownloader 2.0\folderwatch`.

**JDownloader must have the Folder Watch extension enabled** for auto-import to work
(Settings -> Extensions -> Folder Watch). The deployed file appears in the LinkGrabber, collected
but not started.

## Exit codes

- `0` success (no books skipped)
- `1` completed, but some books were skipped (no usable host link) — see the `.skipped.json` report
- `2` config / I/O error (e.g. Folder Watch dir missing on deploy)
- `3` invalid input (missing file, malformed JSON, unknown `--host`)

## Behavior notes

- ddownload links without a filename in the URL get no `filename=` (JDownloader resolves the
  real name); these are counted in the summary.
- Titles are sanitized for Windows-illegal characters; colliding filenames get ` (2)`, ` (3)`, ...
- The container is **collect-only**: links land in the LinkGrabber (`autoStart`/`autoConfirm` off).
  You can then merge packages and start downloads in JDownloader.

## Development

```
cargo test          # run all unit + integration tests
cargo run -- tests/fixtures/books_small.json --no-deploy -o .\.crawljobs\smoke.crawljob
```
