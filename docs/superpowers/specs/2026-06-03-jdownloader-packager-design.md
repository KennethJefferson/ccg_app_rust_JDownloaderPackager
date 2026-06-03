# JDownloader Packager — Design Spec

**Date:** 2026-06-03
**Status:** Draft for user review
**Author:** Claude (Opus 4.8)

---

## 1. Summary

A standalone **Rust CLI** (`jdpackager`) that converts a scraper-produced `books.json` into a
JDownloader **`.crawljob`** link container. It writes the container to a local artifact folder and
copies it into JDownloader's **Folder Watch** directory, where the JDownloader Folder Watch extension
auto-imports the links into the LinkGrabber.

**Core value:** each downloaded file is renamed to the clean book **Title** (via the crawljob
`filename=` key), so after download the user never manually renames files. Exactly one download link is
selected per book using a host-preference chain tuned for the user's RealDebrid setup.

```
books.json  ──▶  jdpackager  ──▶  <app>\.crawljobs\<section>.crawljob          (kept artifact)
                              └─▶  %LOCALAPPDATA%\JDownloader 2.0\folderwatch\  (auto-imports)
```

The input `books.json` is produced by the separate scraper project
(`E:\Workspace.Dev.ClaudeCode.Skills\scraper-sanetst`). This tool is the downstream consumer.

---

## 2. Goals & Non-Goals

### Goals
- Convert one `books.json` into one valid `.crawljob` (properties form).
- Rename each downloaded file to its clean book Title via `filename=` (the central requirement).
- Select exactly one link per book via a configurable host-preference chain.
- Deliver the container both as a kept local artifact and into JDownloader's Folder Watch folder.
- Report what was written and what was skipped (and why).
- Deterministic, testable core (pure modules unit-tested with golden output).

### Non-Goals (v1)
- Audiobooks / multi-part items. Each book is treated as a single file from a single chosen host.
  Genuinely multi-part items (audiobooks) are handled manually by the user; this tool does not attempt
  to bundle multi-part downloads.
- The JSON-array form of `.crawljob`. We use the properties form only.
- DLC / RSDF / CCF or any encrypted container format.
- Driving a running JDownloader via its HTTP API. Delivery is file-based (Folder Watch) only.
- Auto-starting or auto-confirming downloads. The container is collect-only.
- Mirror/fallback links inside one package. One link per book by design (see §4).

---

## 3. Input: `books.json`

Produced by the scraper. Shape (real production sample, 800 entries):

```json
{
  "_meta": {
    "profile": "sanet.st",
    "profile_hash": "sha256:...",
    "schema_version": 1,
    "section": "Computer & Internet",
    "section_url": "https://sanet.st/books/tag/computers-internet-programming",
    "scraped_at": "2026-06-02T12:26:18Z",
    "scraper_version": "0.1.0",
    "book_count": 800,
    "failed_count": 0
  },
  "books": [
    {
      "title": "Ultimate CI/CD for Platform Engineering",
      "isbn": "9349887118",
      "isbn_verified": false,
      "source_url": "https://sanet.st/books/5613390-ultimate-ci-cd-platform-engineering",
      "download_links": {
        "rapidgator": ["https://rapidgator.net/file/9111.../Ultimate_CICD_for_Platform_Engineering.sanet.st.epub.html"],
        "nitroflare": ["https://nitroflare.com/view/6B2.../Ultimate_CICD_for_Platform_Engineering.sanet.st.epub"]
      },
      "all_links": ["...rapidgator...", "...nitroflare..."]
    }
  ]
}
```

Fields this tool reads:
- `_meta.section` — used as the output filename and the shared `packageName` (§4, §5). **Required.**
  If absent or empty, fall back to the input file's stem (e.g. `books`) and warn.
- `books[].title` — the clean name; becomes the rename target. **Required per book**; a book with an
  empty/missing title is skipped and reported.
- `books[].download_links` — a map of `host -> [urls]`. Hosts observed: `rapidgator`, `nitroflare`,
  `ddownload`. Each host list normally has exactly one URL. **Required per book** (a book with no usable
  link is skipped).
- `books[].source_url` — carried into the skipped report for traceability.
- `all_links`, `isbn`, `isbn_verified` — ignored by this tool.

Unknown extra fields are ignored (forward-compatible deserialization).

---

## 4. Host Selection (one link per book)

For each book, build an **ordered preference list of hosts**, then pick the **first host that has a
non-empty link list**, and from it the **first URL**. Exactly one link is chosen per book. This keeps
every crawljob entry single-link, which is what makes `filename=` work (§6).

**Preference chain:**
- No `--host`: `rapidgator → ddownload → skip`
- `--host X`: `X → rapidgator → ddownload → skip` — X is moved to the front, the remainder follows in
  default order, deduplicated. If none of the chained hosts has a link, the book is skipped.
  - `--host rg` → `rapidgator → ddownload → skip` (same as default)
  - `--host nf` → `nitroflare → rapidgator → ddownload → skip`
  - `--host dd` → `ddownload → rapidgator → skip`

**Rationale (user's RealDebrid setup):** rapidgator works ~99.9% of the time and is the default head;
ddownload is the RealDebrid-compatible fallback; nitroflare mostly fails with RealDebrid today, so it is
**never auto-selected** but remains available via explicit `--host nf` (e.g. for a future dedicated NF
account).

**Host aliases (case-insensitive):** `rg` = rapidgator, `nf` = nitroflare, `dd` = ddownload.
The full host name is also accepted (e.g. `--host rapidgator`).

**Edge cases:**
- A host key present but with an empty array → treated as "no link for that host"; continue down the chain.
- A host with multiple URLs → take the first; log a debug note (unexpected for this data).
- An unknown host key in `download_links` (not rapidgator/nitroflare/ddownload) → ignored by selection
  unless explicitly named via `--host <thatname>`; the matcher is driven by the chain, so only chained
  hosts are considered.
- A book whose only links are on hosts not in the chain → skipped and reported.

---

## 5. Output: container structure

One `.crawljob` file, **properties form**, with one job entry per selected book. Multiple jobs are
separated by a literal line `->NEW ENTRY<-`.

All books share a **single `packageName`** equal to `_meta.section` (e.g. `Computer & Internet`). This
pre-groups every book into one JDownloader package on import, so the user's manual "merge into one
package" step is reduced to nearly nothing. The book Title is **not** the package name — it is the
**filename** (§6), which is the actual requirement (the downloaded file on disk is named after the book).

Per-book entry:

```properties
->NEW ENTRY<-
text=https://rapidgator.net/file/9111.../Ultimate_CICD_for_Platform_Engineering.sanet.st.epub.html
filename=Ultimate CI-CD for Platform Engineering.epub
packageName=Computer & Internet
autoConfirm=FALSE
autoStart=FALSE
enabled=TRUE
```

Field decisions:
- `text=` — the single chosen URL, **verbatim** (no decoding, no quoting), so JDownloader's host plugin
  resolves it.
- `filename=` — sanitized `Title.<ext>` (§6). **Omitted** only in the no-extension case (§6.3).
- `packageName=` — `_meta.section`, shared across all entries.
- `autoConfirm=FALSE`, `autoStart=FALSE` — collect into LinkGrabber only; the user reviews and starts.
- `enabled=TRUE`.
- `downloadFolder` — omitted by default; files go to JDownloader's configured default download
  directory. Because files are renamed to `Title.ext`, they are identifiable wherever they land. If the
  user passes `--download-folder <path>` (§7), that value is written into `downloadFolder=` on every
  entry instead.

File encoding: **UTF-8** (no BOM). Line endings: `\n` (JDownloader accepts LF; keep it simple and
consistent). One trailing newline at end of file.

> **Open verification item (V1):** confirm against a live JDownloader Folder Watch import whether the
> properties parser wants a blank line around `->NEW ENTRY<-`, and whether `packageName` repeated
> identically across entries reliably collapses into a single package. The format and separator are
> taken from JDownloader's `folderwatchV2` source; exact whitespace handling is the one detail to
> validate empirically during the build. See §11.

---

## 6. Filename rename & sanitization

Each entry sets `filename=<sanitized Title>.<ext>` so the file downloads already named after the book.

### 6.1 Extension derivation
Derive the extension from the **chosen link's URL filename**:
- Take the last path segment of the URL.
- Strip a trailing host-wrapper `.html` (rapidgator serves pages like `...epub.html` → real ext `.epub`).
- The real extension is the last remaining dotted segment (`.epub`, `.pdf`, `.rar`, `.zip`, etc.).
- We only need the **extension** from the URL; the **stem** is always the sanitized Title.

Examples:
- `Ultimate_CICD_for_Platform_Engineering.sanet.st.epub.html` → ext `.epub`
- `sanet.stMacLifeNr.072026.pdf.html` → ext `.pdf`
- `sanet.st-Mac_Life_-_Nr.07_2026.pdf` → ext `.pdf`

### 6.2 Title → filename sanitization (Windows)
Replace Windows-illegal characters `/ \ : * ? " < > |` with readable equivalents:

| Char | Replacement |
|---|---|
| `/` | `-` |
| `\` | `-` |
| `:` | ` -` (space-dash, so `Title: Subtitle` → `Title - Subtitle`) |
| `*` | (removed) |
| `?` | (removed) |
| `"` | `'` |
| `<` | (removed) |
| `>` | (removed) |
| `\|` | `-` |

Then:
- Collapse runs of whitespace to a single space.
- Trim leading/trailing whitespace.
- Trim trailing dots and trailing spaces (Windows forbids trailing `.`/space in names).
- Guard against Windows **reserved device names** (case-insensitive): `CON, PRN, AUX, NUL, COM1–COM9,
  LPT1–LPT9`. If the sanitized stem equals one of these, append `_` (e.g. `CON` → `CON_`).
- Enforce a maximum length on the **stem** so `stem + ext` stays well under the path limit. Truncate the
  stem to a safe cap (e.g. 150 chars) on a char boundary if needed; the extension is always preserved.

Example: `Ultimate CI/CD for Platform Engineering` → `Ultimate CI-CD for Platform Engineering.epub`.

### 6.3 No-extension case (ddownload)
Some ddownload URLs carry no filename/extension (e.g. `https://ddownload.com/eehlr7pqfmwi`). When the
chosen link yields **no derivable extension**, **omit the `filename=` key** for that entry and let
JDownloader resolve the real filename after it queries the host. Consequence: that one file keeps the
host-resolved name, not the clean Title; the user can rename it manually if desired.

This is a small subset: it only affects ddownload-sourced books whose URL lacks a filename. Rapidgator
URLs (the ~99.9% case) always contain the filename, so they always get the clean rename. These
"filename omitted" entries are **counted and surfaced** in the run summary (§8).

> Design note: we deliberately do **not** borrow the extension from a sibling host link. The user chose
> "let JDownloader resolve it" for simplicity and correctness over cleverness. A future enhancement
> could borrow a sibling extension, but it is out of scope for v1.

### 6.4 Duplicate filename collisions
Two different books can sanitize to the same `Title.ext` (e.g. titles differing only by an illegal
char, or by truncation). On import into a single shared package, identical filenames could cause
JDownloader to treat them as duplicates or append ` (1)`. The tool detects sanitized-filename
collisions **within a run** and disambiguates deterministically by appending ` (2)`, ` (3)`, … to the
stem of the later colliding entries, and reports the count of disambiguated names in the summary. (First
occurrence keeps the clean name.)

---

## 7. CLI interface & configuration

```
jdpackager <input> [--host <rg|nf|dd|name>] [-o <path>] [--no-deploy] [--download-folder <path>]
```

- `<input>` — path to a `books.json` file, **or** a directory containing one (the tool looks for
  `books.json` inside). **Required.**
- `--host <h>` — preferred host placed at the head of the selection chain (§4). Optional; omit for the
  default `rapidgator → ddownload → skip`.
- `-o <path>` — override the kept-artifact output path. Default:
  `<app>\.crawljobs\<sanitized section>.crawljob`.
- `--no-deploy` — write only the local artifact; skip copying into the Folder Watch folder.
- `--download-folder <path>` — optional; if given, set `downloadFolder=` in every entry. Default:
  omitted (JDownloader default dir).

`<app>` = the directory containing the `jdpackager` executable.

### 7.1 Config file
`config.toml`, located next to the executable:

```toml
# JDownloader Folder Watch directory. The generated .crawljob is copied here to auto-import.
folderwatch_dir = "C:\\Users\\<you>\\AppData\\Local\\JDownloader 2.0\\folderwatch"
```

- If `config.toml` is absent or `folderwatch_dir` is unset, default to
  `%LOCALAPPDATA%\JDownloader 2.0\folderwatch`.
- On deploy (i.e. unless `--no-deploy`), if the resolved Folder Watch directory does not exist, **fail
  with a specific, actionable error** instructing the user to set `folderwatch_dir` in `config.toml`.
  The kept local artifact is still written first (so work is never lost) before the deploy step runs.

---

## 8. Output reporting & skip handling

On completion, print a summary to stdout, e.g.:

```
Computer & Internet: 800 books → 793 entries written
  by host:   rapidgator 770, ddownload 23, nitroflare 0
  skipped:   7 (no usable host link)
  filename omitted (JD resolves): 23 (ddownload, no extension)
  disambiguated filenames: 2
Artifact: E:\Workspace.Dev.Rust\JdownloaderPackager\.crawljobs\Computer & Internet.crawljob
Deployed: C:\Users\<you>\AppData\Local\JDownloader 2.0\folderwatch\Computer & Internet.crawljob
```

**Skipped report:** books skipped for lack of a usable link are written to
`<app>\.crawljobs\<sanitized section>.skipped.json`:

```json
{
  "section": "Computer & Internet",
  "generated_from": "E:\\...\\books.json",
  "skipped": [
    { "title": "Some Audiobook Title", "source_url": "https://sanet.st/...", "reason": "no link on rapidgator/ddownload" }
  ]
}
```

If there are zero skips, the skipped report file is **not written**, and the summary shows
`skipped: 0`.

---

## 9. Architecture (modules)

Small, focused modules in idiomatic Rust:

| Module | Responsibility | Purity |
|---|---|---|
| `main.rs` | CLI parsing (clap), orchestration, exit codes | impure (I/O, args) |
| `config.rs` | Load `config.toml`; resolve Folder Watch dir + default | impure (reads file/env) |
| `models.rs` | `serde` types for `books.json` (`Meta`, `Book`, links map) | pure (data) |
| `hosts.rs` | Host aliases; build preference chain; pick one link per book | **pure** |
| `filename.rs` | Extension derivation; Title sanitization; collision disambiguation | **pure** |
| `crawljob.rs` | Render one entry and the whole file from selected items | **pure** |
| `output.rs` | Atomic write of artifact; deploy-copy to watch dir; write skipped report | impure (I/O) |

Data flow: `main` → `config` → parse `books.json` (`models`) → for each book `hosts::pick` +
`filename::build` → `crawljob::render` → `output::write_artifact` + `output::deploy` +
`output::write_skipped`.

### 9.1 Crates
- `clap` (derive) — CLI.
- `serde` + `serde_json` — input parsing.
- `toml` — config.
- `anyhow` (or `thiserror` for typed errors) — error handling.
- `directories` or `std::env` — resolve `%LOCALAPPDATA%`.
- (Std `fs` for atomic write; no extra crate required.)

---

## 10. Error handling & atomic writes

**Explicit errors, no silent failures.** Specific messages for:
- Input path missing / not a file / directory without a `books.json`.
- Malformed JSON (surface the serde error location).
- Missing `_meta.section` (warn + fall back to input stem).
- Unwritable artifact directory.
- Folder Watch directory missing on deploy.

**Atomic writes (both artifact and deploy):**
1. Write the artifact to a temp file in the **same directory** as the final artifact, then `rename` into
   place (atomic on the same volume).
2. For deploy, write to a temp file **inside the Folder Watch directory** (e.g.
   `.<name>.crawljob.tmp`), then `rename` to the final `<name>.crawljob`. This guarantees the Folder
   Watch poller never observes a partially written file — the watched name appears only as a complete
   file in one atomic step. (A plain copy is **not** safe here because the watcher could read mid-copy.)
   - If the watch dir is on a different volume than the temp source, still write-temp-then-rename
     **within** the watch dir (never copy directly to the final name).

**Exit codes:**
- `0` — success. No books were dropped. (Filename-omitted and disambiguated entries are **normal**
  successful outcomes — they are reported in the summary but do **not** change the exit code, since they
  occur on routine runs.)
- `1` — completed, but one or more books were **skipped** (no usable host link). The run still produced
  a valid container for the rest; `1` signals "review the skipped report."
- `2` — config / I/O error (e.g. watch dir missing on deploy, unwritable path).
- `3` — invalid input (missing file, malformed JSON).

---

## 11. Testing strategy (TDD)

Pure modules are unit-tested first; impure modules get focused tests around their contract.

| Layer | How tested |
|---|---|
| `hosts.rs` | Unit: default chain, `--host` reordering for rg/nf/dd, dedup, empty arrays, host-present-but-empty, multi-URL host, unknown host, skip when none. |
| `filename.rs` | Unit: ext derivation incl. `.epub.html`, `.pdf.html`, bare `.pdf`, no-extension (ddownload); every illegal-char replacement; whitespace collapse; trailing dot/space trim; reserved names (`CON`→`CON_`); length truncation; collision disambiguation. |
| `crawljob.rs` | Unit: single-entry render; multi-entry with `->NEW ENTRY<-`; `filename=` omitted case; **golden-output** test for a small fixture. |
| `output.rs` | Unit/integration: atomic write produces complete file; deploy uses temp-then-rename in watch dir; skipped report shape; empty-skip behavior. |
| `models.rs` | Unit: deserialize the real `books.json`; tolerate unknown fields; missing/null `isbn`; missing `section`. |
| end-to-end | Integration: run against the real 800-entry `books.json` fixture; assert entry count, host tallies, and that output parses back into the expected structure. |

**Live verification (manual, during build):** import a generated `.crawljob` into a real JDownloader
via Folder Watch and confirm (a) all entries import, (b) the single shared `packageName` collapses into
one package, (c) `filename=` renames are applied, (d) collect-only behavior (no auto-start). This
resolves Open Verification item V1 in §5.

---

## 12. Build plan

1. Scaffold the Rust project (`cargo new`), add crates, set up `cargo test`.
2. TDD the pure core in dependency order: `models` → `hosts` → `filename` → `crawljob`.
3. Implement impure edges: `config`, `output` (atomic write + deploy), `main` wiring + clap.
4. Integration test against the real `books.json` fixture.
5. Live JDownloader Folder Watch verification (§11); resolve §5 V1 (whitespace / package grouping).
6. Finalize README + `config.toml` template.

---

## 13. Open Items Carried Into Build

### Resolved by live JDownloader verification (2026-06-03)
- **V1 (§5): RESOLVED.** A real Folder Watch import of the 800-book run confirmed: the `->NEW ENTRY<-`
  format (LF line endings, no blank lines) imports correctly; all 669 entries collapsed into a single
  package via the identical repeated `packageName`; collect-only held (nothing auto-started). The
  imported `.crawljob` was moved to `folderwatch\added\` as expected.
- **LF line endings: RESOLVED.** JDownloader accepted the LF-only `.crawljob` without issue.
- **Empty-skip file behavior: RESOLVED** — implemented as "do not write when zero skips."
- **Default watch path: RESOLVED** — on the user's install the watched folder is the plain
  `%LOCALAPPDATA%\JDownloader 2.0\folderwatch` (JD's `FolderWatch: Folders` = `["folderwatch"]`,
  relative to the JD home), which matches this tool's default. (An unrelated `__folderwatch` folder
  exists on the same machine and is NOT the watched folder — do not point config at it.)

### Known gap (routed upstream — NOT a jdpackager defect)
- **~107 of 669 books are not Title-renamed** because their rapidgator links use the short
  `/file/<hash>` form with **no filename/extension anywhere in any of the book's links** (sibling-borrow
  is not viable — only 2 of 109 had a sibling extension). jdpackager correctly omits `filename=` in this
  case and JDownloader resolves the host name. The real filename/extension is not present in
  `books.json`. **Decision:** fix upstream — the scraper should capture the extension from the sanet.st
  page and record it in `books.json`. When it does, jdpackager will need a small change to read the
  extension from that new field as a fallback when the URL lacks one (the ~560 books with the long-form
  URL already rename correctly today).
