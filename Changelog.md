# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-06-03

Initial release.

### Added
- Rust CLI `jdpackager` that converts a scraper-produced `books.json` into a JDownloader
  `.crawljob` link container.
- Per-book single-link selection with a configurable host-preference chain
  (default `rapidgator -> ddownload -> skip`; `--host <rg|nf|dd|name>` moves a host to the
  head of the chain). `nitroflare` is never auto-selected.
- Filename rewriting: each downloaded file is renamed to the clean book Title via the crawljob
  `filename=` key, including derivation of the extension from the link URL (strips rapidgator's
  `.html` wrapper) and Windows-safe sanitization of the Title (illegal-character replacement,
  reserved-name guard, length cap, collision disambiguation).
- Single shared `packageName` (the section) so all books import into one JDownloader package.
- Collect-only import (`autoStart=FALSE`, `autoConfirm=FALSE`).
- Atomic artifact write, plus atomic deploy into JDownloader's Folder Watch directory
  (temp-then-rename inside the watch dir so the watcher never reads a partial file).
- `config.toml` support for the Folder Watch directory, defaulting to
  `%LOCALAPPDATA%\JDownloader 2.0\folderwatch`.
- Run summary and a `<name>.skipped.json` report for books with no usable host link.
- CLI flags: `--host`, `-o/--out`, `--no-deploy`, `--download-folder`.
- Exit codes: `0` success, `1` completed with skips, `2` config/IO error, `3` invalid input.
- Documentation: `README.md`, `Usage.md`, `CLAUDE.md`, design spec, and implementation plan.

### Known issues
- Books whose links lack any file extension (e.g. rapidgator short `/file/<hash>` URLs) are
  emitted without a `filename=`; JDownloader resolves the host name instead of the Title. The
  fix is upstream: the scraper should capture the file extension into `books.json`.

[0.1.0]: https://semver.org/spec/v2.0.0.html
