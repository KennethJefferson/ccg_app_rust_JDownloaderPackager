# Usage

`jdpackager` converts a scraper-produced `books.json` into a JDownloader `.crawljob`
link container and (optionally) drops it into JDownloader's Folder Watch directory for
automatic import.

## Synopsis

```
jdpackager <input> [--host <rg|nf|dd|name>] [-o <path>] [--no-deploy] [--download-folder <path>]
```

## Arguments

| Argument | Description |
|---|---|
| `<input>` | Path to a `books.json` file, or a directory that contains one. **Required.** |
| `--host <h>` | Preferred download host, placed at the head of the selection chain. Aliases: `rg` (rapidgator), `nf` (nitroflare), `dd` (ddownload). Full names also accepted. |
| `-o, --out <path>` | Override the kept-artifact output path. Default: `<exe dir>\.crawljobs\<section>.crawljob`. |
| `--no-deploy` | Write only the local artifact; do not copy into the Folder Watch folder. |
| `--download-folder <path>` | Set `downloadFolder=` on every entry (default: JDownloader's own default download directory). |

## How a link is chosen

Exactly **one** download link is selected per book, so each downloaded file can be renamed
to its clean Title via the crawljob `filename=` key.

The host preference chain:

- **No `--host`:** `rapidgator -> ddownload -> skip`
- **`--host X`:** `X -> rapidgator -> ddownload -> skip` (X first, then the default order, deduplicated)

`nitroflare` is never auto-selected; request it explicitly with `--host nf`. A book with no
link on any host in the chain is **skipped** and recorded in the skipped report.

## Output

- **Artifact:** the `.crawljob` is written to `<exe dir>\.crawljobs\<section>.crawljob`
  (or your `-o` path). It is written atomically and always before any deploy step, so work
  is never lost.
- **Deploy:** unless `--no-deploy`, a copy is placed in JDownloader's Folder Watch folder
  (atomic temp-then-rename, so the watcher never sees a partial file). JDownloader imports it
  and moves the file into a `added\` subfolder.
- **Skipped report:** if any books were skipped, a `<name>.skipped.json` is written next to
  the artifact listing each skipped book's title, source URL, and reason.

## Configuration

`jdpackager` finds JDownloader's Folder Watch folder via `config.toml`, located next to the
executable. Copy `config.toml.example` to `config.toml` and set the path:

```toml
folderwatch_dir = "C:\\Users\\YOURNAME\\AppData\\Local\\JDownloader 2.0\\folderwatch"
```

If `config.toml` is absent, the default `%LOCALAPPDATA%\JDownloader 2.0\folderwatch` is used.

**JDownloader must have the Folder Watch extension enabled** (Settings -> Extensions -> Folder
Watch) for auto-import to work. Imported links land in the LinkGrabber, collected but not
started.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success, no books skipped. |
| `1` | Completed, but one or more books were skipped (no usable host link) — see the `.skipped.json` report. |
| `2` | Config / I/O error (e.g. the Folder Watch directory does not exist on deploy). |
| `3` | Invalid input (missing file, malformed JSON, unknown `--host`). |

## Examples

Convert and auto-deploy to JDownloader (default chain, rapidgator preferred):

```
jdpackager "C:\runs\comp-internet\books.json"
```

Convert only, no deploy, to a chosen path:

```
jdpackager books.json --no-deploy -o D:\containers\comp.crawljob
```

Prefer nitroflare (e.g. with a dedicated NF account), falling back to rapidgator then ddownload:

```
jdpackager books.json --host nf
```

Point all downloads at a specific folder:

```
jdpackager books.json --download-folder "D:\Books\Computer & Internet"
```

## Notes

- A book becomes one entry; all entries share a single `packageName` (the section from
  `_meta.section`), so they import into one JDownloader package that you can then merge/rename
  and start as you like.
- Files whose URL has no extension (e.g. rapidgator short `/file/<hash>` links) are emitted
  without a `filename=` and JDownloader resolves the real name; these are counted in the
  summary as "filename omitted".
- Titles are sanitized for Windows-illegal characters; filenames that would collide within a
  run get ` (2)`, ` (3)`, ... appended.
