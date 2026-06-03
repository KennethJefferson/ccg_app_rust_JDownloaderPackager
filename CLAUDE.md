# CLAUDE.md

Guidance for Claude Code when working in this repository.

## What this is

`jdpackager` — a Rust CLI that converts a scraper-produced `books.json` into a JDownloader
`.crawljob` link container and copies it into JDownloader's Folder Watch directory for
automatic import. The defining requirement: each downloaded file is renamed to the clean book
**Title** via the crawljob `filename=` key, so files never need manual renaming after download.

Input comes from a sibling scraper project (`E:\Workspace.Dev.ClaudeCode.Skills\scraper-sanetst`).
This tool is the downstream consumer.

## Build / test / run

```
cargo build              # build
cargo test               # all tests (unit + integration); expect everything green
cargo clippy --all-targets   # lints; keep this clean
cargo run -- tests/fixtures/books_small.json --no-deploy -o .\.crawljobs\smoke.crawljob
```

Note: on some shells `cargo` is not on PATH; it lives at `~/.cargo/bin/cargo`.

## Architecture

Flat `src/*.rs` modules, declared in `main.rs`. The core is pure and unit-tested; the edges do I/O.

| Module | Responsibility | Purity |
|---|---|---|
| `main.rs` | CLI orchestration, exit codes | impure |
| `cli.rs` | clap `Args` + pure helpers (input resolution, section name, summary) | mixed |
| `models.rs` | serde types for `books.json` (`BooksFile`, `Meta`, `Book`) | pure |
| `hosts.rs` | host alias resolution + preference chain + pick one link | pure |
| `filename.rs` | extension derivation, Title sanitization, collision disambiguation | pure |
| `select.rs` | glue: per-book selection into entries/skips + tallies | pure |
| `crawljob.rs` | render entries into `.crawljob` properties text | pure |
| `config.rs` | load `config.toml`, resolve Folder Watch dir | impure |
| `output.rs` | atomic artifact write, deploy to watch dir, skipped report | impure |

Data flow: `main` → `config` → parse `books.json` (`models`) → `select_all` (`hosts` + `filename`)
→ `crawljob::render` → `output` (atomic write + deploy + skipped report).

## Key design decisions (don't regress these)

- **One link per book** (host chain: default `rapidgator -> ddownload -> skip`; `--host X` puts X
  first). This is what makes `filename=` work — JDownloader's `filename=` only applies to
  single-link crawljob jobs.
- **Single shared `packageName`** (= `_meta.section`) so all books import into one package.
- **Collect-only**: `autoStart=FALSE`, `autoConfirm=FALSE`.
- **Atomic writes**: temp-then-rename, and for deploy the temp is created *inside* the watch dir
  so Folder Watch never sees a partial file.
- **`.crawljob` properties form**, entries separated by `->NEW ENTRY<-`, UTF-8, LF line endings.
  Verified accepted by a real JDownloader import (groups into one package; file moves to
  `folderwatch\added\`).
- nitroflare is never auto-selected (it mostly fails with the user's RealDebrid); only via `--host nf`.

## Known gap (do not "fix" downstream)

~107 of ~669 books (mostly magazines) keep raw host filenames instead of the Title, because their
rapidgator links use the short `/file/<hash>` form with **no extension anywhere in the book's
links** (sibling-borrow is not viable — verified). `jdpackager` correctly omits `filename=` and
JDownloader resolves the name. The real fix is **upstream in the scraper**: it should capture the
file extension from the sanet.st page into `books.json`. When that lands, `jdpackager` will need a
small change to read the extension from the new field as a fallback when the URL has none.

## Conventions

- Type annotations everywhere; handle errors explicitly (`anyhow`); prefer explicit over implicit.
- Self-documenting code, minimal comments.
- TDD: tests alongside each module; golden-output test for the crawljob renderer.
- Keep `cargo clippy` clean (no `#[allow(...)]` to paper over real lints).

## Specs & plans

- Design spec: `docs/superpowers/specs/2026-06-03-jdownloader-packager-design.md`
- Implementation plan: `docs/superpowers/plans/2026-06-03-jdownloader-packager.md`
