# JDownloader Packager Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standalone Rust CLI (`jdpackager`) that converts a scraper-produced `books.json` into a JDownloader `.crawljob` link container, renaming each downloaded file to its clean book Title.

**Architecture:** A small CLI with a pure core (parse → select one link per book → derive filename → render crawljob text) and impure edges (load config, atomic file writes, deploy to JDownloader's Folder Watch folder). Pure modules are unit-tested with golden output; impure modules tested against temp dirs.

**Tech Stack:** Rust (edition 2021), `clap` (derive) for CLI, `serde`/`serde_json` for input, `toml` for config, `anyhow` for errors, `tempfile` for tests. Flat `src/*.rs` module layout (modules declared in `main.rs`), matching sibling projects in `E:\Workspace.Dev.Rust\`.

**Reference spec:** `docs/superpowers/specs/2026-06-03-jdownloader-packager-design.md`

**Environment notes:**
- Windows 11, PowerShell. `cargo` is installed at `C:\Users\Tony Baloney\.cargo\bin\cargo.exe` (version 1.93.0). In an interactive terminal it is on PATH, so plain `cargo` commands work. If a command reports `cargo` not found, prefix with the full path.
- Real integration fixture (800 books) lives at:
  `E:\Workspace.Dev.ClaudeCode.Skills\scraper-sanetst\__files.ccg\__permanent\runs\sanet.st\comp-internet\books.json`
  We copy a trimmed + full copy into `tests/fixtures/` in Task 2 so tests are self-contained.
- Git repo already initialized (branch `main`); the spec is already committed.

**Conventions for every task:**
- TDD: write the failing test, run it (confirm it fails for the right reason), implement minimally, run it (confirm pass), commit.
- Run a single test with: `cargo test <test_name>` ; run a module's tests with `cargo test <module>::`.
- Commit messages use Conventional Commits (`feat:`, `test:`, `chore:`, `docs:`).
- All modules are declared in `src/main.rs` with `mod <name>;`. Add the `mod` line the first time a module file is created (called out in steps).

---

## Task 0: Project scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `.gitignore` (already exists — verify only)
- Create: `rust-toolchain` is NOT needed.

- [ ] **Step 1: Initialize the Cargo project**

The directory already exists and is a git repo. Create the Cargo project in place. Run:

```
cargo init --name jdpackager --bin
```

Expected: creates `Cargo.toml` and `src/main.rs`. If it complains the directory isn't empty, that's fine — it only adds the two files. If `src/main.rs` already exists from init, we overwrite it in Step 3.

- [ ] **Step 2: Write `Cargo.toml`**

Replace the generated `Cargo.toml` with:

```toml
[package]
name = "jdpackager"
version = "0.1.0"
edition = "2021"
description = "Convert a scraper books.json into a JDownloader .crawljob link container"

[[bin]]
name = "jdpackager"
path = "src/main.rs"

[dependencies]
anyhow = "1.0"
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

[dev-dependencies]
tempfile = "3.10"
```

- [ ] **Step 3: Write a minimal `src/main.rs` that compiles**

```rust
fn main() {
    println!("jdpackager");
}
```

- [ ] **Step 4: Verify it builds**

Run: `cargo build`
Expected: compiles successfully (downloads dependencies on first run). No warnings about missing crates.

- [ ] **Step 5: Verify `.gitignore` covers build + runtime output**

Confirm `.gitignore` contains `/target`, `.crawljobs/`, `config.toml`, and `.superpowers/`. It should already (created during brainstorming). If any are missing, add them.

- [ ] **Step 6: Commit**

```
git add Cargo.toml Cargo.lock src/main.rs .gitignore
git commit -m "chore: scaffold jdpackager Cargo project"
```

---

## Task 1: Input models (`models.rs`)

Defines the `serde` types for `books.json`. `download_links` is a map of host → list of URLs; order within the file is not relied upon (we drive ordering from the host chain in Task 2). We use `IndexMap`? No — std `HashMap` is fine since selection is chain-driven, not map-order-driven.

**Files:**
- Create: `src/models.rs`
- Modify: `src/main.rs` (add `mod models;`)
- Test: inline `#[cfg(test)]` in `src/models.rs`

- [ ] **Step 1: Declare the module in `src/main.rs`**

Add at the top of `src/main.rs` (above `fn main`):

```rust
mod models;
```

- [ ] **Step 2: Write the failing test**

Create `src/models.rs` with the types and a test that deserializes a representative JSON snippet:

```rust
use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BooksFile {
    #[serde(rename = "_meta")]
    pub meta: Meta,
    pub books: Vec<Book>,
}

#[derive(Debug, Deserialize)]
pub struct Meta {
    /// Section name; used for output filename and shared packageName.
    /// Optional in the type so a malformed/absent value degrades gracefully (handled by caller).
    #[serde(default)]
    pub section: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Book {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    /// host -> list of urls
    #[serde(default)]
    pub download_links: HashMap<String, Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_meta_and_books() {
        let json = r#"
        {
          "_meta": { "profile": "sanet.st", "section": "Computer & Internet", "book_count": 1 },
          "books": [
            {
              "title": "Ultimate CI/CD for Platform Engineering",
              "isbn": "9349887118",
              "isbn_verified": false,
              "source_url": "https://sanet.st/books/5613390",
              "download_links": {
                "rapidgator": ["https://rapidgator.net/file/abc/x.epub.html"],
                "nitroflare": ["https://nitroflare.com/view/DEF/x.epub"]
              },
              "all_links": ["a", "b"]
            }
          ]
        }
        "#;

        let parsed: BooksFile = serde_json::from_str(json).expect("should parse");
        assert_eq!(parsed.meta.section.as_deref(), Some("Computer & Internet"));
        assert_eq!(parsed.books.len(), 1);
        let book = &parsed.books[0];
        assert_eq!(book.title.as_deref(), Some("Ultimate CI/CD for Platform Engineering"));
        assert_eq!(book.download_links["rapidgator"].len(), 1);
    }

    #[test]
    fn tolerates_missing_optional_fields() {
        // null isbn, missing source_url, missing section, unknown extra field
        let json = r#"
        {
          "_meta": { "profile": "x", "extra_unknown": 42 },
          "books": [
            { "title": "No Links Book", "isbn": null, "download_links": {} }
          ]
        }
        "#;

        let parsed: BooksFile = serde_json::from_str(json).expect("should parse");
        assert_eq!(parsed.meta.section, None);
        assert_eq!(parsed.books[0].title.as_deref(), Some("No Links Book"));
        assert!(parsed.books[0].source_url.is_none());
        assert!(parsed.books[0].download_links.is_empty());
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail or pass**

Run: `cargo test models::`
Expected: Because the implementation IS the types above, these should PASS once it compiles. If there is a compile error, fix it. (This task's "implementation" and "test" are written together because serde types are declarative; the test is the meaningful verification.)

- [ ] **Step 4: Confirm no dead-code warnings break the build**

Run: `cargo build`
Expected: compiles. Unused-field warnings are acceptable at this stage (fields are consumed in later tasks). If warnings are noisy, do NOT add `#[allow(dead_code)]` yet — later tasks use these fields.

- [ ] **Step 5: Commit**

```
git add src/main.rs src/models.rs
git commit -m "feat: add books.json serde models"
```

---

## Task 2: Test fixtures

Copy the real `books.json` and create a small hand-made fixture for fast unit/golden tests.

**Files:**
- Create: `tests/fixtures/books_small.json` (hand-authored, 3 books covering the key cases)
- Create: `tests/fixtures/books_real.json` (copy of the real 800-book file)

- [ ] **Step 1: Copy the real fixture**

Run (PowerShell):

```
New-Item -ItemType Directory -Force tests\fixtures | Out-Null
Copy-Item "E:\Workspace.Dev.ClaudeCode.Skills\scraper-sanetst\__files.ccg\__permanent\runs\sanet.st\comp-internet\books.json" "tests\fixtures\books_real.json"
```

Expected: `tests/fixtures/books_real.json` exists (~678 KB).

- [ ] **Step 2: Create the small fixture**

Create `tests/fixtures/books_small.json` with three books exercising: (a) rapidgator+nitroflare, (b) all three hosts incl. ddownload-no-extension, (c) a book with no rapidgator/ddownload (nitroflare only → skipped by default chain):

```json
{
  "_meta": {
    "profile": "sanet.st",
    "schema_version": 1,
    "section": "Computer & Internet",
    "book_count": 3,
    "failed_count": 0
  },
  "books": [
    {
      "title": "Ultimate CI/CD for Platform Engineering",
      "isbn": "9349887118",
      "isbn_verified": false,
      "source_url": "https://sanet.st/books/5613390-ultimate-ci-cd-platform-engineering",
      "download_links": {
        "rapidgator": ["https://rapidgator.net/file/9111b76c/Ultimate_CICD_for_Platform_Engineering.sanet.st.epub.html"],
        "nitroflare": ["https://nitroflare.com/view/6B2DBAB/Ultimate_CICD_for_Platform_Engineering.sanet.st.epub"]
      },
      "all_links": []
    },
    {
      "title": "Mac Life - Nr.07 2026",
      "isbn": null,
      "isbn_verified": false,
      "source_url": "https://sanet.st/books/magazines/5613099-mac-life-nr07-2026",
      "download_links": {
        "rapidgator": ["https://rapidgator.net/file/d223089e/sanet.stMacLifeNr.072026.pdf.html"],
        "nitroflare": ["https://nitroflare.com/view/963B4D4/sanet.st-Mac_Life_-_Nr.07_2026.pdf"],
        "ddownload": ["https://ddownload.com/eehlr7pqfmwi"]
      },
      "all_links": []
    },
    {
      "title": "Nitroflare Only Book",
      "isbn": null,
      "isbn_verified": false,
      "source_url": "https://sanet.st/books/9999999-nitro-only",
      "download_links": {
        "nitroflare": ["https://nitroflare.com/view/ZZZ/nitro_only.sanet.st.epub"]
      },
      "all_links": []
    }
  ]
}
```

- [ ] **Step 3: Sanity-check the small fixture parses**

Add a temporary test to `src/models.rs` tests module:

```rust
    #[test]
    fn parses_small_fixture_file() {
        let text = std::fs::read_to_string("tests/fixtures/books_small.json").expect("read fixture");
        let parsed: BooksFile = serde_json::from_str(&text).expect("parse fixture");
        assert_eq!(parsed.books.len(), 3);
        assert_eq!(parsed.meta.section.as_deref(), Some("Computer & Internet"));
    }
```

Run: `cargo test parses_small_fixture_file`
Expected: PASS. (Tests run from the crate root, so the relative path `tests/fixtures/...` resolves.)

- [ ] **Step 4: Commit**

```
git add tests/fixtures/books_small.json tests/fixtures/books_real.json src/models.rs
git commit -m "test: add small and real books.json fixtures"
```

---

## Task 3: Host selection (`hosts.rs`)

Pure module: alias resolution, building the preference chain, and picking exactly one link per book.

**Files:**
- Create: `src/hosts.rs`
- Modify: `src/main.rs` (add `mod hosts;`)
- Test: inline `#[cfg(test)]` in `src/hosts.rs`

The canonical host keys (as they appear in `download_links`) are `rapidgator`, `nitroflare`, `ddownload`. The default chain is `[rapidgator, ddownload]` (note: nitroflare is NOT in the default chain). An explicit `--host X` puts X at the head, then appends the default chain, deduped.

- [ ] **Step 1: Declare the module**

Add to `src/main.rs`:

```rust
mod hosts;
```

- [ ] **Step 2: Write failing tests for alias resolution**

Create `src/hosts.rs`:

```rust
//! Pure host-selection logic: alias resolution, preference chain, pick one link.

/// Canonical host keys as they appear in books.json `download_links`.
pub const RAPIDGATOR: &str = "rapidgator";
pub const NITROFLARE: &str = "nitroflare";
pub const DDOWNLOAD: &str = "ddownload";

/// Resolve a user-supplied --host value (alias or full name, case-insensitive)
/// to a canonical host key. Returns None if unrecognized.
pub fn resolve_alias(input: &str) -> Option<&'static str> {
    match input.trim().to_ascii_lowercase().as_str() {
        "rg" | "rapidgator" => Some(RAPIDGATOR),
        "nf" | "nitroflare" => Some(NITROFLARE),
        "dd" | "ddownload" => Some(DDOWNLOAD),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_aliases_case_insensitively() {
        assert_eq!(resolve_alias("rg"), Some(RAPIDGATOR));
        assert_eq!(resolve_alias("RG"), Some(RAPIDGATOR));
        assert_eq!(resolve_alias("rapidgator"), Some(RAPIDGATOR));
        assert_eq!(resolve_alias("nf"), Some(NITROFLARE));
        assert_eq!(resolve_alias("Nitroflare"), Some(NITROFLARE));
        assert_eq!(resolve_alias("dd"), Some(DDOWNLOAD));
        assert_eq!(resolve_alias(" ddownload "), Some(DDOWNLOAD));
        assert_eq!(resolve_alias("mega"), None);
    }
}
```

- [ ] **Step 3: Run alias test**

Run: `cargo test hosts::tests::resolves_aliases_case_insensitively`
Expected: PASS (implementation written alongside).

- [ ] **Step 4: Write failing test for the preference chain**

Add to the `tests` module in `src/hosts.rs`:

```rust
    #[test]
    fn default_chain_is_rapidgator_then_ddownload() {
        assert_eq!(preference_chain(None), vec![RAPIDGATOR, DDOWNLOAD]);
    }

    #[test]
    fn host_rg_matches_default() {
        assert_eq!(preference_chain(Some(RAPIDGATOR)), vec![RAPIDGATOR, DDOWNLOAD]);
    }

    #[test]
    fn host_nf_puts_nitroflare_first_then_default() {
        assert_eq!(preference_chain(Some(NITROFLARE)), vec![NITROFLARE, RAPIDGATOR, DDOWNLOAD]);
    }

    #[test]
    fn host_dd_puts_ddownload_first_deduped() {
        assert_eq!(preference_chain(Some(DDOWNLOAD)), vec![DDOWNLOAD, RAPIDGATOR]);
    }
```

- [ ] **Step 5: Implement `preference_chain`**

Add to `src/hosts.rs` (above the tests module):

```rust
/// The default preference order. nitroflare is intentionally absent (never auto-selected).
const DEFAULT_CHAIN: [&str; 2] = [RAPIDGATOR, DDOWNLOAD];

/// Build the ordered host preference list. If `preferred` is Some, it goes first,
/// then the default chain follows, with duplicates removed (preserving first occurrence).
pub fn preference_chain(preferred: Option<&'static str>) -> Vec<&'static str> {
    let mut chain: Vec<&'static str> = Vec::new();
    if let Some(p) = preferred {
        chain.push(p);
    }
    for &h in DEFAULT_CHAIN.iter() {
        if !chain.contains(&h) {
            chain.push(h);
        }
    }
    chain
}
```

- [ ] **Step 6: Run chain tests**

Run: `cargo test hosts::tests`
Expected: all chain tests PASS.

- [ ] **Step 7: Write failing test for picking a link**

Add to the `tests` module:

```rust
    use std::collections::HashMap;

    fn links(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(h, urls)| (h.to_string(), urls.iter().map(|u| u.to_string()).collect()))
            .collect()
    }

    #[test]
    fn picks_first_available_in_chain() {
        let dl = links(&[
            ("rapidgator", &["RG_URL"]),
            ("nitroflare", &["NF_URL"]),
        ]);
        // default chain rg->dd: should pick rapidgator
        let picked = pick_link(&dl, None);
        assert_eq!(picked, Some(("rapidgator", "RG_URL".to_string())));
    }

    #[test]
    fn falls_back_to_ddownload_when_no_rapidgator() {
        let dl = links(&[
            ("nitroflare", &["NF_URL"]),
            ("ddownload", &["DD_URL"]),
        ]);
        let picked = pick_link(&dl, None);
        assert_eq!(picked, Some(("ddownload", "DD_URL".to_string())));
    }

    #[test]
    fn skips_when_only_offchain_host() {
        let dl = links(&[("nitroflare", &["NF_URL"])]);
        // default chain has no nitroflare -> None
        assert_eq!(pick_link(&dl, None), None);
    }

    #[test]
    fn host_nf_picks_nitroflare_when_present() {
        let dl = links(&[
            ("rapidgator", &["RG_URL"]),
            ("nitroflare", &["NF_URL"]),
        ]);
        let picked = pick_link(&dl, Some(NITROFLARE));
        assert_eq!(picked, Some(("nitroflare", "NF_URL".to_string())));
    }

    #[test]
    fn empty_host_array_is_skipped_in_chain() {
        let dl = links(&[
            ("rapidgator", &[]),
            ("ddownload", &["DD_URL"]),
        ]);
        let picked = pick_link(&dl, None);
        assert_eq!(picked, Some(("ddownload", "DD_URL".to_string())));
    }

    #[test]
    fn multiple_urls_takes_first() {
        let dl = links(&[("rapidgator", &["FIRST", "SECOND"])]);
        let picked = pick_link(&dl, None);
        assert_eq!(picked, Some(("rapidgator", "FIRST".to_string())));
    }
```

- [ ] **Step 8: Implement `pick_link`**

Add to `src/hosts.rs` (above the tests module):

```rust
use std::collections::HashMap;

/// Pick exactly one (host, url) for a book given its download_links and an optional
/// preferred host. Walks the preference chain and returns the first host that has a
/// non-empty URL list, taking that list's first URL. Returns None if no chained host
/// has a usable link (the book is skipped by the caller).
pub fn pick_link(
    download_links: &HashMap<String, Vec<String>>,
    preferred: Option<&'static str>,
) -> Option<(&'static str, String)> {
    for host in preference_chain(preferred) {
        if let Some(urls) = download_links.get(host) {
            if let Some(first) = urls.first() {
                return Some((host, first.clone()));
            }
        }
    }
    None
}
```

Note: there is a duplicate `use std::collections::HashMap;` — one at module top (added here) and one inside `tests`. Keep the module-level `use` for `pick_link`; the test module's `use super::*;` already brings it in, so REMOVE the `use std::collections::HashMap;` line inside the tests module (the helper `links` uses the same import via `super`). Verify it compiles.

- [ ] **Step 9: Run all host tests**

Run: `cargo test hosts::`
Expected: all PASS.

- [ ] **Step 10: Commit**

```
git add src/main.rs src/hosts.rs
git commit -m "feat: add host preference chain and single-link selection"
```

---

## Task 4: Filename derivation & sanitization (`filename.rs`)

Pure module: derive the file extension from a chosen URL, sanitize a Title into a Windows-safe stem, and disambiguate collisions across a run.

**Files:**
- Create: `src/filename.rs`
- Modify: `src/main.rs` (add `mod filename;`)
- Test: inline `#[cfg(test)]` in `src/filename.rs`

- [ ] **Step 1: Declare the module**

Add to `src/main.rs`:

```rust
mod filename;
```

- [ ] **Step 2: Write failing tests for extension derivation**

Create `src/filename.rs`:

```rust
//! Pure filename logic: extension derivation, Title sanitization, collision disambiguation.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_epub_from_rapidgator_html_wrapper() {
        assert_eq!(
            derive_extension("https://rapidgator.net/file/abc/Ultimate_CICD.sanet.st.epub.html"),
            Some("epub".to_string())
        );
    }

    #[test]
    fn derives_pdf_from_html_wrapper() {
        assert_eq!(
            derive_extension("https://rapidgator.net/file/d2/sanet.stMacLifeNr.072026.pdf.html"),
            Some("pdf".to_string())
        );
    }

    #[test]
    fn derives_pdf_from_bare_url() {
        assert_eq!(
            derive_extension("https://nitroflare.com/view/963/sanet.st-Mac_Life_-_Nr.07_2026.pdf"),
            Some("pdf".to_string())
        );
    }

    #[test]
    fn no_extension_for_ddownload_id_url() {
        assert_eq!(derive_extension("https://ddownload.com/eehlr7pqfmwi"), None);
    }

    #[test]
    fn no_extension_when_last_segment_has_no_dot() {
        assert_eq!(derive_extension("https://example.com/path/file"), None);
    }

    #[test]
    fn ignores_query_and_fragment() {
        assert_eq!(
            derive_extension("https://host.com/a/file.epub?token=x#frag"),
            Some("epub".to_string())
        );
    }
}
```

- [ ] **Step 3: Implement `derive_extension`**

Add to `src/filename.rs` (above the tests module):

```rust
/// Known host-wrapper suffix that hides the real extension (rapidgator serves `...epub.html`).
const WRAPPER_EXT: &str = "html";

/// Derive the real file extension (without the dot) from a download URL.
/// Strips query/fragment, takes the last path segment, removes a trailing `.html`
/// wrapper, then returns the final dotted segment. Returns None if there is no
/// usable extension (e.g. ddownload id-only URLs).
pub fn derive_extension(url: &str) -> Option<String> {
    // Strip fragment then query.
    let no_frag = url.split('#').next().unwrap_or(url);
    let no_query = no_frag.split('?').next().unwrap_or(no_frag);

    // Last path segment.
    let segment = no_query.rsplit('/').next().unwrap_or("");
    if segment.is_empty() {
        return None;
    }

    // Split on dots; drop a trailing "html" wrapper.
    let mut parts: Vec<&str> = segment.split('.').collect();
    if parts.len() >= 2 && parts.last().map(|s| s.eq_ignore_ascii_case(WRAPPER_EXT)) == Some(true) {
        parts.pop();
    }

    // Need at least stem + ext to have a real extension.
    if parts.len() < 2 {
        return None;
    }

    let ext = parts.last().unwrap();
    if ext.is_empty() {
        return None;
    }
    Some(ext.to_ascii_lowercase())
}
```

- [ ] **Step 4: Run extension tests**

Run: `cargo test filename::tests` — only the extension tests exist so far.
Expected: all PASS.

- [ ] **Step 5: Write failing tests for sanitization**

Add to the `tests` module in `src/filename.rs`:

```rust
    #[test]
    fn replaces_slash_with_dash() {
        assert_eq!(sanitize_stem("Ultimate CI/CD for Platform Engineering"),
                   "Ultimate CI-CD for Platform Engineering");
    }

    #[test]
    fn replaces_backslash_and_pipe_with_dash() {
        assert_eq!(sanitize_stem("A\\B|C"), "A-B-C");
    }

    #[test]
    fn colon_becomes_space_dash() {
        assert_eq!(sanitize_stem("Title: Subtitle"), "Title - Subtitle");
    }

    #[test]
    fn removes_star_question_angles() {
        assert_eq!(sanitize_stem("What? <Really>* Yes"), "What Really Yes");
    }

    #[test]
    fn quote_becomes_apostrophe() {
        assert_eq!(sanitize_stem("The \"Best\" Book"), "The 'Best' Book");
    }

    #[test]
    fn collapses_whitespace_and_trims() {
        assert_eq!(sanitize_stem("  Too   many    spaces  "), "Too many spaces");
    }

    #[test]
    fn trims_trailing_dots_and_spaces() {
        assert_eq!(sanitize_stem("Trailing dots... "), "Trailing dots");
    }

    #[test]
    fn reserved_name_gets_suffixed() {
        assert_eq!(sanitize_stem("CON"), "CON_");
        assert_eq!(sanitize_stem("con"), "con_");
        assert_eq!(sanitize_stem("Lpt1"), "Lpt1_");
    }

    #[test]
    fn long_stem_is_truncated_to_cap() {
        let long = "a".repeat(300);
        let out = sanitize_stem(&long);
        assert!(out.chars().count() <= 150, "stem should be capped at 150 chars");
    }

    #[test]
    fn empty_after_sanitize_falls_back() {
        // A title made entirely of removed chars should not yield an empty stem.
        assert_eq!(sanitize_stem("***"), "untitled");
        assert_eq!(sanitize_stem("   "), "untitled");
    }
```

- [ ] **Step 6: Implement `sanitize_stem`**

Add to `src/filename.rs` (above the tests module):

```rust
/// Maximum length (in chars) for the sanitized stem, leaving room for the extension
/// well under the Windows MAX_PATH constraint.
const MAX_STEM_CHARS: usize = 150;

/// Windows reserved device base names (compared case-insensitively against the stem).
const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Sanitize a Title into a Windows-safe filename stem (no extension).
pub fn sanitize_stem(title: &str) -> String {
    // 1. Character replacement.
    let mut out = String::with_capacity(title.len());
    for ch in title.chars() {
        match ch {
            '/' | '\\' | '|' => out.push('-'),
            ':' => out.push_str(" -"),
            '"' => out.push('\''),
            '*' | '?' | '<' | '>' => { /* removed */ }
            _ => out.push(ch),
        }
    }

    // 2. Collapse whitespace runs to a single space, trim ends.
    let collapsed = out.split_whitespace().collect::<Vec<_>>().join(" ");

    // 3. Trim trailing dots and spaces (Windows forbids them).
    let trimmed = collapsed.trim_end_matches(|c: char| c == '.' || c == ' ').to_string();

    // 4. Fall back if empty.
    let mut result = if trimmed.is_empty() { "untitled".to_string() } else { trimmed };

    // 5. Truncate to cap on a char boundary.
    if result.chars().count() > MAX_STEM_CHARS {
        result = result.chars().take(MAX_STEM_CHARS).collect();
        // Re-trim trailing space/dot that truncation may have exposed.
        result = result.trim_end_matches(|c: char| c == '.' || c == ' ').to_string();
        if result.is_empty() {
            result = "untitled".to_string();
        }
    }

    // 6. Reserved-name guard (whole-stem match, case-insensitive).
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(&result)) {
        result.push('_');
    }

    result
}
```

- [ ] **Step 7: Run sanitization tests**

Run: `cargo test filename::tests`
Expected: all extension + sanitization tests PASS.

- [ ] **Step 8: Write failing test for building a full filename**

Add to the `tests` module:

```rust
    #[test]
    fn build_filename_combines_stem_and_ext() {
        assert_eq!(
            build_filename("Ultimate CI/CD for Platform Engineering", Some("epub")),
            Some("Ultimate CI-CD for Platform Engineering.epub".to_string())
        );
    }

    #[test]
    fn build_filename_none_when_no_ext() {
        assert_eq!(build_filename("Some Title", None), None);
    }
```

- [ ] **Step 9: Implement `build_filename`**

Add to `src/filename.rs` (above the tests module):

```rust
/// Build the full `filename=` value `<sanitized stem>.<ext>`.
/// Returns None when there is no extension (caller then omits the filename= key).
pub fn build_filename(title: &str, ext: Option<&str>) -> Option<String> {
    let ext = ext?;
    Some(format!("{}.{}", sanitize_stem(title), ext))
}
```

- [ ] **Step 10: Write failing test for collision disambiguation**

Add to the `tests` module:

```rust
    #[test]
    fn disambiguator_appends_counter_for_duplicates() {
        let mut d = Disambiguator::new();
        assert_eq!(d.unique("Book.epub"), "Book.epub");
        assert_eq!(d.unique("Book.epub"), "Book (2).epub");
        assert_eq!(d.unique("Book.epub"), "Book (3).epub");
        // Different name is untouched.
        assert_eq!(d.unique("Other.pdf"), "Other.pdf");
    }

    #[test]
    fn disambiguator_is_case_insensitive() {
        let mut d = Disambiguator::new();
        assert_eq!(d.unique("Book.epub"), "Book.epub");
        assert_eq!(d.unique("book.EPUB"), "book (2).EPUB");
    }
```

- [ ] **Step 11: Implement `Disambiguator`**

Add to `src/filename.rs` (above the tests module):

```rust
use std::collections::HashSet;

/// Tracks filenames already used in a run and produces a unique variant by appending
/// ` (2)`, ` (3)`, ... to the stem (before the extension) on collision. Case-insensitive.
pub struct Disambiguator {
    seen: HashSet<String>,
}

impl Disambiguator {
    pub fn new() -> Self {
        Self { seen: HashSet::new() }
    }

    /// Returns a unique filename, recording it as used. `name` is a full `stem.ext`
    /// (or just a stem if it has no dot).
    pub fn unique(&mut self, name: &str) -> String {
        if self.insert_if_new(name) {
            return name.to_string();
        }
        let (stem, ext) = split_name(name);
        let mut n = 2;
        loop {
            let candidate = match ext {
                Some(e) => format!("{} ({}).{}", stem, n, e),
                None => format!("{} ({})", stem, n),
            };
            if self.insert_if_new(&candidate) {
                return candidate;
            }
            n += 1;
        }
    }

    fn insert_if_new(&mut self, name: &str) -> bool {
        self.seen.insert(name.to_ascii_lowercase())
    }
}

impl Default for Disambiguator {
    fn default() -> Self {
        Self::new()
    }
}

/// Split a filename into (stem, Option<ext>) on the LAST dot. A leading dot or no dot
/// yields ext = None.
fn split_name(name: &str) -> (&str, Option<&str>) {
    match name.rfind('.') {
        Some(idx) if idx > 0 && idx < name.len() - 1 => (&name[..idx], Some(&name[idx + 1..])),
        _ => (name, None),
    }
}
```

- [ ] **Step 12: Run all filename tests**

Run: `cargo test filename::`
Expected: all PASS.

- [ ] **Step 13: Commit**

```
git add src/main.rs src/filename.rs
git commit -m "feat: add filename extension derivation, sanitization, and disambiguation"
```

---

## Task 5: Selection assembly (`select.rs`)

Glue the pure pieces into a per-book decision, producing either a renderable Entry or a Skip. This isolates the "what goes in the file" decision from rendering and I/O, and is the natural home for the run summary tallies.

**Files:**
- Create: `src/select.rs`
- Modify: `src/main.rs` (add `mod select;`)
- Test: inline `#[cfg(test)]` in `src/select.rs`

- [ ] **Step 1: Declare the module**

Add to `src/main.rs`:

```rust
mod select;
```

- [ ] **Step 2: Write failing tests**

Create `src/select.rs`:

```rust
//! Assemble per-book selection results from the pure host/filename logic.

use crate::filename::{build_filename, derive_extension, Disambiguator};
use crate::hosts::pick_link;
use crate::models::Book;

/// A book that will become one crawljob entry.
#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    pub url: String,
    pub host: &'static str,
    /// The `filename=` value, or None if no extension could be derived (key omitted).
    pub filename: Option<String>,
}

/// A book that was dropped, with a reason for the report.
#[derive(Debug, PartialEq, Eq)]
pub struct Skip {
    pub title: String,
    pub source_url: Option<String>,
    pub reason: String,
}

/// Outcome of selecting over all books.
#[derive(Debug, Default)]
pub struct Selection {
    pub entries: Vec<Entry>,
    pub skips: Vec<Skip>,
    /// Count of entries whose filename= was omitted (no extension).
    pub filename_omitted: usize,
    /// Count of entries whose filename was disambiguated due to collision.
    pub disambiguated: usize,
    /// host -> count of entries chosen from that host.
    pub host_counts: std::collections::BTreeMap<&'static str, usize>,
}

/// Run selection across all books with the given preferred host.
pub fn select_all(books: &[Book], preferred: Option<&'static str>) -> Selection {
    let mut sel = Selection::default();
    let mut disamb = Disambiguator::new();

    for book in books {
        let title = book.title.as_deref().unwrap_or("").trim().to_string();
        if title.is_empty() {
            sel.skips.push(Skip {
                title: String::new(),
                source_url: book.source_url.clone(),
                reason: "missing title".to_string(),
            });
            continue;
        }

        match pick_link(&book.download_links, preferred) {
            Some((host, url)) => {
                let ext = derive_extension(&url);
                let filename = match build_filename(&title, ext.as_deref()) {
                    Some(name) => {
                        let unique = disamb.unique(&name);
                        if unique != name {
                            sel.disambiguated += 1;
                        }
                        Some(unique)
                    }
                    None => {
                        sel.filename_omitted += 1;
                        None
                    }
                };
                *sel.host_counts.entry(host).or_insert(0) += 1;
                sel.entries.push(Entry { url, host, filename });
            }
            None => {
                sel.skips.push(Skip {
                    title,
                    source_url: book.source_url.clone(),
                    reason: "no link on rapidgator/ddownload".to_string(),
                });
            }
        }
    }

    sel
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn book(title: &str, links: &[(&str, &str)]) -> Book {
        let mut dl: HashMap<String, Vec<String>> = HashMap::new();
        for (h, u) in links {
            dl.entry(h.to_string()).or_default().push(u.to_string());
        }
        Book {
            title: Some(title.to_string()),
            source_url: Some(format!("https://src/{}", title.replace(' ', "_"))),
            download_links: dl,
        }
    }

    #[test]
    fn selects_entries_and_skips_with_default_chain() {
        let books = vec![
            book("Alpha", &[("rapidgator", "https://rapidgator.net/file/a/Alpha.epub.html")]),
            book("Beta NF Only", &[("nitroflare", "https://nitroflare.com/v/B/Beta.epub")]),
        ];
        let sel = select_all(&books, None);
        assert_eq!(sel.entries.len(), 1);
        assert_eq!(sel.entries[0].host, "rapidgator");
        assert_eq!(sel.entries[0].filename.as_deref(), Some("Alpha.epub"));
        assert_eq!(sel.skips.len(), 1);
        assert_eq!(sel.skips[0].title, "Beta NF Only");
        assert_eq!(*sel.host_counts.get("rapidgator").unwrap(), 1);
    }

    #[test]
    fn omits_filename_for_ddownload_no_extension() {
        let books = vec![book("Gamma", &[("ddownload", "https://ddownload.com/eehlr7pqfmwi")])];
        let sel = select_all(&books, None);
        assert_eq!(sel.entries.len(), 1);
        assert_eq!(sel.entries[0].host, "ddownload");
        assert_eq!(sel.entries[0].filename, None);
        assert_eq!(sel.filename_omitted, 1);
    }

    #[test]
    fn disambiguates_colliding_titles() {
        let books = vec![
            book("Same", &[("rapidgator", "https://rapidgator.net/file/1/Same.epub.html")]),
            book("Same", &[("rapidgator", "https://rapidgator.net/file/2/Same.epub.html")]),
        ];
        let sel = select_all(&books, None);
        assert_eq!(sel.entries.len(), 2);
        assert_eq!(sel.entries[0].filename.as_deref(), Some("Same.epub"));
        assert_eq!(sel.entries[1].filename.as_deref(), Some("Same (2).epub"));
        assert_eq!(sel.disambiguated, 1);
    }

    #[test]
    fn missing_title_is_skipped() {
        let b = Book { title: None, source_url: None, download_links: HashMap::new() };
        let sel = select_all(&[b], None);
        assert_eq!(sel.entries.len(), 0);
        assert_eq!(sel.skips.len(), 1);
        assert_eq!(sel.skips[0].reason, "missing title");
    }
}
```

- [ ] **Step 3: Run selection tests**

Run: `cargo test select::`
Expected: all PASS, no warnings.

- [ ] **Step 4: Commit**

```
git add src/main.rs src/select.rs
git commit -m "feat: assemble per-book selection (entries, skips, tallies)"
```

---

## Task 6: Crawljob rendering (`crawljob.rs`)

Pure module: render `Entry` values into the `.crawljob` properties text, with golden-output tests.

**Files:**
- Create: `src/crawljob.rs`
- Modify: `src/main.rs` (add `mod crawljob;`)
- Test: inline `#[cfg(test)]` in `src/crawljob.rs`

The rendered shape per entry (from spec §5), separator line `->NEW ENTRY<-` before each entry:

```
->NEW ENTRY<-
text=<url>
filename=<name>        (omitted if None)
packageName=<section>
downloadFolder=<path>  (omitted unless provided)
autoConfirm=FALSE
autoStart=FALSE
enabled=TRUE
```

- [ ] **Step 1: Declare the module**

Add to `src/main.rs`:

```rust
mod crawljob;
```

- [ ] **Step 2: Write failing golden tests**

Create `src/crawljob.rs`:

```rust
//! Render selected entries into JDownloader .crawljob properties text.

use crate::select::Entry;

/// Options that apply to every entry in the file.
pub struct RenderOptions<'a> {
    pub package_name: &'a str,
    pub download_folder: Option<&'a str>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(url: &str, filename: Option<&str>, host: &'static str) -> Entry {
        Entry { url: url.to_string(), host, filename: filename.map(|s| s.to_string()) }
    }

    #[test]
    fn renders_single_entry_with_filename() {
        let entries = vec![entry(
            "https://rapidgator.net/file/9111/Ultimate.sanet.st.epub.html",
            Some("Ultimate CI-CD for Platform Engineering.epub"),
            "rapidgator",
        )];
        let opts = RenderOptions { package_name: "Computer & Internet", download_folder: None };
        let out = render(&entries, &opts);
        let expected = "\
->NEW ENTRY<-
text=https://rapidgator.net/file/9111/Ultimate.sanet.st.epub.html
filename=Ultimate CI-CD for Platform Engineering.epub
packageName=Computer & Internet
autoConfirm=FALSE
autoStart=FALSE
enabled=TRUE
";
        assert_eq!(out, expected);
    }

    #[test]
    fn omits_filename_line_when_none() {
        let entries = vec![entry("https://ddownload.com/eehlr7pqfmwi", None, "ddownload")];
        let opts = RenderOptions { package_name: "Computer & Internet", download_folder: None };
        let out = render(&entries, &opts);
        let expected = "\
->NEW ENTRY<-
text=https://ddownload.com/eehlr7pqfmwi
packageName=Computer & Internet
autoConfirm=FALSE
autoStart=FALSE
enabled=TRUE
";
        assert_eq!(out, expected);
    }

    #[test]
    fn includes_download_folder_when_provided() {
        let entries = vec![entry("https://rapidgator.net/file/x/a.epub.html", Some("A.epub"), "rapidgator")];
        let opts = RenderOptions { package_name: "Sec", download_folder: Some("D:\\Books") };
        let out = render(&entries, &opts);
        assert!(out.contains("downloadFolder=D:\\Books\n"));
        // Order: downloadFolder after packageName, before autoConfirm.
        let pkg_idx = out.find("packageName=Sec").unwrap();
        let df_idx = out.find("downloadFolder=").unwrap();
        let ac_idx = out.find("autoConfirm=").unwrap();
        assert!(pkg_idx < df_idx && df_idx < ac_idx);
    }

    #[test]
    fn renders_multiple_entries_each_with_separator() {
        let entries = vec![
            entry("URL1", Some("One.epub"), "rapidgator"),
            entry("URL2", Some("Two.pdf"), "rapidgator"),
        ];
        let opts = RenderOptions { package_name: "S", download_folder: None };
        let out = render(&entries, &opts);
        assert_eq!(out.matches("->NEW ENTRY<-").count(), 2);
        assert!(out.contains("text=URL1"));
        assert!(out.contains("text=URL2"));
    }

    #[test]
    fn empty_entries_render_empty_string() {
        let opts = RenderOptions { package_name: "S", download_folder: None };
        assert_eq!(render(&[], &opts), "");
    }
}
```

- [ ] **Step 3: Implement `render`**

Add to `src/crawljob.rs` (above the tests module):

```rust
/// Render all entries into the full .crawljob file body (UTF-8, LF line endings).
/// Each entry is preceded by a `->NEW ENTRY<-` line. Returns an empty string when
/// there are no entries.
pub fn render(entries: &[Entry], opts: &RenderOptions) -> String {
    let mut out = String::new();
    for e in entries {
        out.push_str("->NEW ENTRY<-\n");
        out.push_str(&format!("text={}\n", e.url));
        if let Some(name) = &e.filename {
            out.push_str(&format!("filename={}\n", name));
        }
        out.push_str(&format!("packageName={}\n", opts.package_name));
        if let Some(df) = opts.download_folder {
            out.push_str(&format!("downloadFolder={}\n", df));
        }
        out.push_str("autoConfirm=FALSE\n");
        out.push_str("autoStart=FALSE\n");
        out.push_str("enabled=TRUE\n");
    }
    out
}
```

- [ ] **Step 4: Run crawljob tests**

Run: `cargo test crawljob::`
Expected: all PASS. (Golden strings use `\` literally in the `download_folder` test — Rust raw vs normal: the test uses `"D:\\Books"` which is the string `D:\Books`; `assert!(out.contains("downloadFolder=D:\\Books\n"))` checks `downloadFolder=D:\Books` followed by newline. Confirm pass.)

- [ ] **Step 5: Commit**

```
git add src/main.rs src/crawljob.rs
git commit -m "feat: render selected entries to .crawljob properties text"
```

---

## Task 7: Config loading (`config.rs`)

Impure module: resolve the Folder Watch directory from `config.toml` (next to the executable) with a `%LOCALAPPDATA%` default.

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs` (add `mod config;`)
- Test: inline `#[cfg(test)]` in `src/config.rs`

- [ ] **Step 1: Declare the module**

Add to `src/main.rs`:

```rust
mod config;
```

- [ ] **Step 2: Write failing tests for parsing + default resolution**

Create `src/config.rs`:

```rust
//! Load config.toml (next to the executable) and resolve the Folder Watch directory.

use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub folderwatch_dir: Option<String>,
}

/// Parse config TOML text into a Config. Missing/empty -> default (all None).
pub fn parse_config(text: &str) -> anyhow::Result<Config> {
    let cfg: Config = toml::from_str(text)?;
    Ok(cfg)
}

/// Resolve the effective Folder Watch directory:
/// 1. config.folderwatch_dir if set and non-empty,
/// 2. else `<local_app_data>/JDownloader 2.0/folderwatch`.
/// `local_app_data` is passed in for testability (caller supplies env in production).
pub fn resolve_folderwatch_dir(cfg: &Config, local_app_data: Option<&str>) -> Option<PathBuf> {
    if let Some(dir) = cfg.folderwatch_dir.as_deref() {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    local_app_data.map(|lad| {
        let mut p = PathBuf::from(lad);
        p.push("JDownloader 2.0");
        p.push("folderwatch");
        p
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_explicit_dir() {
        let cfg = parse_config("folderwatch_dir = \"D:\\\\JD\\\\folderwatch\"").unwrap();
        assert_eq!(cfg.folderwatch_dir.as_deref(), Some("D:\\JD\\folderwatch"));
    }

    #[test]
    fn empty_toml_yields_none() {
        let cfg = parse_config("").unwrap();
        assert!(cfg.folderwatch_dir.is_none());
    }

    #[test]
    fn explicit_dir_takes_precedence() {
        let cfg = Config { folderwatch_dir: Some("D:\\Custom".to_string()) };
        let resolved = resolve_folderwatch_dir(&cfg, Some("C:\\Users\\me\\AppData\\Local"));
        assert_eq!(resolved, Some(PathBuf::from("D:\\Custom")));
    }

    #[test]
    fn default_uses_local_app_data() {
        let cfg = Config::default();
        let resolved = resolve_folderwatch_dir(&cfg, Some("C:\\Users\\me\\AppData\\Local")).unwrap();
        let s = resolved.to_string_lossy().replace('/', "\\");
        assert!(s.ends_with("AppData\\Local\\JDownloader 2.0\\folderwatch"), "got {s}");
    }

    #[test]
    fn empty_string_dir_falls_back_to_default() {
        let cfg = Config { folderwatch_dir: Some("   ".to_string()) };
        let resolved = resolve_folderwatch_dir(&cfg, Some("C:\\LAD"));
        assert_eq!(resolved, Some(PathBuf::from("C:\\LAD\\JDownloader 2.0\\folderwatch")));
    }

    #[test]
    fn no_local_app_data_and_no_config_yields_none() {
        let cfg = Config::default();
        assert_eq!(resolve_folderwatch_dir(&cfg, None), None);
    }
}
```

- [ ] **Step 3: Run config tests**

Run: `cargo test config::`
Expected: all PASS.

- [ ] **Step 4: Add `load_config_beside_exe` (thin filesystem wrapper)**

Add to `src/config.rs` (above the tests module). This is the impure entry point used by `main`; it is not unit-tested (it touches the real exe path), but it reuses the tested `parse_config`:

```rust
use std::fs;

/// Load config.toml located next to the current executable. If the file is absent,
/// returns a default Config. Errors only on a present-but-unparseable file.
pub fn load_config_beside_exe() -> anyhow::Result<Config> {
    let exe = std::env::current_exe()?;
    let dir = exe.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
    let path = dir.join("config.toml");
    match fs::read_to_string(&path) {
        Ok(text) => parse_config(&text),
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(anyhow::anyhow!("failed to read {}: {e}", path.display())),
    }
}
```

- [ ] **Step 5: Verify build**

Run: `cargo build`
Expected: compiles. `load_config_beside_exe` may be unused until Task 9 — that's acceptable; it will be wired in `main`. If the dead-code warning is bothersome, leave it; Task 9 removes it by using the function.

- [ ] **Step 6: Commit**

```
git add src/main.rs src/config.rs
git commit -m "feat: add config.toml loading and folderwatch dir resolution"
```

---

## Task 8: Output writing (`output.rs`)

Impure module: atomic artifact write, atomic deploy into the watch dir (temp-then-rename **inside** the watch dir), and skipped-report writing.

**Files:**
- Create: `src/output.rs`
- Modify: `src/main.rs` (add `mod output;`)
- Test: inline `#[cfg(test)]` in `src/output.rs` (uses `tempfile`)

- [ ] **Step 1: Declare the module**

Add to `src/main.rs`:

```rust
mod output;
```

- [ ] **Step 2: Write failing tests for atomic write + skipped report**

Create `src/output.rs`:

```rust
//! Filesystem output: atomic artifact write, deploy to watch dir, skipped report.

use std::fs;
use std::path::{Path, PathBuf};

use crate::select::Skip;

/// Write `content` to `final_path` atomically: write a sibling temp file in the same
/// directory, then rename over the destination. Creates parent dirs as needed.
pub fn atomic_write(final_path: &Path, content: &str) -> anyhow::Result<()> {
    let parent = final_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("path has no parent: {}", final_path.display()))?;
    fs::create_dir_all(parent)?;

    let file_name = final_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("path has no file name: {}", final_path.display()))?
        .to_string_lossy();
    let tmp = parent.join(format!(".{}.tmp", file_name));

    fs::write(&tmp, content.as_bytes())?;
    // rename is atomic on the same volume; overwrites destination on Windows & Unix.
    fs::rename(&tmp, final_path)?;
    Ok(())
}

/// Deploy `content` into the watch directory under `file_name`, atomically. The temp
/// file is created INSIDE the watch dir so the final rename is same-volume and the
/// watcher never sees a partial file. Fails if `watch_dir` does not exist.
pub fn deploy_to_watch(watch_dir: &Path, file_name: &str, content: &str) -> anyhow::Result<PathBuf> {
    if !watch_dir.exists() {
        return Err(anyhow::anyhow!(
            "JDownloader Folder Watch directory does not exist: {}\n\
             Set `folderwatch_dir` in config.toml (next to the executable) to your JDownloader \
             folderwatch path, or pass --no-deploy.",
            watch_dir.display()
        ));
    }
    let final_path = watch_dir.join(file_name);
    let tmp = watch_dir.join(format!(".{}.tmp", file_name));
    fs::write(&tmp, content.as_bytes())?;
    fs::rename(&tmp, &final_path)?;
    Ok(final_path)
}

/// Write the skipped report JSON. Does nothing (and returns Ok(None)) when `skips` is empty.
pub fn write_skipped_report(
    path: &Path,
    section: &str,
    generated_from: &str,
    skips: &[Skip],
) -> anyhow::Result<Option<PathBuf>> {
    if skips.is_empty() {
        return Ok(None);
    }
    let json = build_skipped_json(section, generated_from, skips);
    atomic_write(path, &json)?;
    Ok(Some(path.to_path_buf()))
}

/// Build the skipped-report JSON string (pure; separated for testing).
pub fn build_skipped_json(section: &str, generated_from: &str, skips: &[Skip]) -> String {
    // Build with serde_json to get correct escaping.
    let items: Vec<serde_json::Value> = skips
        .iter()
        .map(|s| {
            serde_json::json!({
                "title": s.title,
                "source_url": s.source_url,
                "reason": s.reason,
            })
        })
        .collect();
    let doc = serde_json::json!({
        "section": section,
        "generated_from": generated_from,
        "skipped": items,
    });
    serde_json::to_string_pretty(&doc).expect("serialize skipped report")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_creates_file_with_content() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sub").join("out.crawljob");
        atomic_write(&target, "hello\nworld\n").unwrap();
        let read = fs::read_to_string(&target).unwrap();
        assert_eq!(read, "hello\nworld\n");
        // No leftover temp file.
        let leftover: Vec<_> = fs::read_dir(target.parent().unwrap())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftover.is_empty(), "temp file should be gone");
    }

    #[test]
    fn atomic_write_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("out.txt");
        atomic_write(&target, "first").unwrap();
        atomic_write(&target, "second").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "second");
    }

    #[test]
    fn deploy_writes_into_existing_watch_dir() {
        let dir = tempfile::tempdir().unwrap();
        let path = deploy_to_watch(dir.path(), "Computer & Internet.crawljob", "BODY").unwrap();
        assert_eq!(path, dir.path().join("Computer & Internet.crawljob"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "BODY");
    }

    #[test]
    fn deploy_errors_when_watch_dir_missing() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does_not_exist");
        let err = deploy_to_watch(&missing, "x.crawljob", "BODY").unwrap_err();
        assert!(err.to_string().contains("does not exist"));
    }

    #[test]
    fn skipped_report_not_written_when_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.skipped.json");
        let res = write_skipped_report(&path, "Sec", "in.json", &[]).unwrap();
        assert!(res.is_none());
        assert!(!path.exists());
    }

    #[test]
    fn skipped_report_written_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.skipped.json");
        let skips = vec![Skip {
            title: "T".to_string(),
            source_url: Some("https://s".to_string()),
            reason: "no link on rapidgator/ddownload".to_string(),
        }];
        let res = write_skipped_report(&path, "Sec", "in.json", &skips).unwrap();
        assert_eq!(res, Some(path.clone()));
        let body = fs::read_to_string(&path).unwrap();
        assert!(body.contains("\"section\": \"Sec\""));
        assert!(body.contains("\"title\": \"T\""));
        assert!(body.contains("no link on rapidgator/ddownload"));
    }

    #[test]
    fn skipped_json_handles_null_source_url() {
        let skips = vec![Skip { title: "T".to_string(), source_url: None, reason: "missing title".to_string() }];
        let json = build_skipped_json("Sec", "in.json", &skips);
        assert!(json.contains("\"source_url\": null"));
    }
}
```

- [ ] **Step 3: Run output tests**

Run: `cargo test output::`
Expected: all PASS.

- [ ] **Step 4: Commit**

```
git add src/main.rs src/output.rs
git commit -m "feat: add atomic artifact write, watch-dir deploy, skipped report"
```

---

## Task 9: CLI wiring (`main.rs` + `cli.rs`)

Wire everything: parse args, resolve input, parse books, select, render, write artifact, deploy, print summary, set exit code.

**Files:**
- Create: `src/cli.rs` (clap args struct + input resolution + section-name derivation)
- Modify: `src/main.rs` (orchestration, exit codes)
- Test: inline `#[cfg(test)]` in `src/cli.rs` for the pure helpers (input resolution, section derivation, summary formatting)

- [ ] **Step 1: Declare the module**

Add to `src/main.rs`:

```rust
mod cli;
```

- [ ] **Step 2: Write failing tests for pure CLI helpers**

Create `src/cli.rs`:

```rust
//! CLI argument definitions and pure helpers (input resolution, section derivation, summary).

use std::path::{Path, PathBuf};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "jdpackager", about = "Convert a scraper books.json into a JDownloader .crawljob")]
pub struct Args {
    /// Path to a books.json file, or a directory containing one.
    pub input: PathBuf,

    /// Preferred host placed at the head of the selection chain (rg|nf|dd or full name).
    #[arg(long)]
    pub host: Option<String>,

    /// Override the kept-artifact output path.
    #[arg(short = 'o', long)]
    pub out: Option<PathBuf>,

    /// Write only the local artifact; skip copying into the Folder Watch folder.
    #[arg(long)]
    pub no_deploy: bool,

    /// If given, set downloadFolder= in every entry.
    #[arg(long)]
    pub download_folder: Option<String>,
}

/// Resolve the input path to an actual books.json file.
/// - If `input` is a file, use it.
/// - If `input` is a directory, look for `books.json` inside it.
pub fn resolve_input_file(input: &Path) -> anyhow::Result<PathBuf> {
    if input.is_file() {
        Ok(input.to_path_buf())
    } else if input.is_dir() {
        let candidate = input.join("books.json");
        if candidate.is_file() {
            Ok(candidate)
        } else {
            Err(anyhow::anyhow!(
                "directory {} does not contain books.json",
                input.display()
            ))
        }
    } else {
        Err(anyhow::anyhow!("input path not found: {}", input.display()))
    }
}

/// Derive the section/base name used for output filenames.
/// Prefers `meta_section`; falls back to the input file's stem; finally "books".
pub fn derive_section_name(meta_section: Option<&str>, input_file: &Path) -> String {
    if let Some(s) = meta_section {
        let t = s.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    input_file
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "books".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_input_finds_file_directly() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("books.json");
        std::fs::write(&f, "{}").unwrap();
        assert_eq!(resolve_input_file(&f).unwrap(), f);
    }

    #[test]
    fn resolve_input_finds_books_json_in_dir() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("books.json");
        std::fs::write(&f, "{}").unwrap();
        assert_eq!(resolve_input_file(dir.path()).unwrap(), f);
    }

    #[test]
    fn resolve_input_errors_on_missing() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.json");
        assert!(resolve_input_file(&missing).is_err());
    }

    #[test]
    fn section_prefers_meta() {
        let p = Path::new("C:\\runs\\comp\\books.json");
        assert_eq!(derive_section_name(Some("Computer & Internet"), p), "Computer & Internet");
    }

    #[test]
    fn section_falls_back_to_stem() {
        let p = Path::new("C:\\runs\\comp\\my_run.json");
        assert_eq!(derive_section_name(None, p), "my_run");
        assert_eq!(derive_section_name(Some("   "), p), "my_run");
    }
}
```

- [ ] **Step 3: Run CLI helper tests**

Run: `cargo test cli::`
Expected: all PASS.

- [ ] **Step 4: Write the summary formatter (pure) with a test**

Add to `src/cli.rs` (above the tests module):

```rust
use crate::select::Selection;

/// Format the run summary block (without the artifact/deploy path lines, which the
/// caller appends since they involve real paths).
pub fn format_summary(section: &str, total_books: usize, sel: &Selection) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "{}: {} books -> {} entries written\n",
        section,
        total_books,
        sel.entries.len()
    ));
    // by host line
    let mut host_parts: Vec<String> = sel
        .host_counts
        .iter()
        .map(|(h, c)| format!("{} {}", h, c))
        .collect();
    if host_parts.is_empty() {
        host_parts.push("(none)".to_string());
    }
    s.push_str(&format!("  by host:   {}\n", host_parts.join(", ")));
    s.push_str(&format!("  skipped:   {}\n", sel.skips.len()));
    s.push_str(&format!(
        "  filename omitted (JD resolves): {}\n",
        sel.filename_omitted
    ));
    s.push_str(&format!("  disambiguated filenames: {}\n", sel.disambiguated));
    s
}
```

Add the test to the `tests` module:

```rust
    use crate::select::Entry;

    #[test]
    fn summary_includes_counts() {
        let mut sel = Selection::default();
        sel.entries.push(Entry { url: "u".into(), host: "rapidgator", filename: Some("A.epub".into()) });
        *sel.host_counts.entry("rapidgator").or_insert(0) += 1;
        sel.filename_omitted = 2;
        let out = format_summary("Computer & Internet", 5, &sel);
        assert!(out.contains("Computer & Internet: 5 books -> 1 entries written"));
        assert!(out.contains("rapidgator 1"));
        assert!(out.contains("filename omitted (JD resolves): 2"));
        assert!(out.contains("skipped:   0"));
    }
```

- [ ] **Step 5: Run the summary test**

Run: `cargo test cli::tests::summary_includes_counts`
Expected: PASS.

- [ ] **Step 6: Write `main.rs` orchestration**

Replace the body of `src/main.rs` below the `mod` declarations with the orchestration. The full file should look like this (keep all `mod` lines at the top):

```rust
mod cli;
mod config;
mod crawljob;
mod filename;
mod hosts;
mod models;
mod output;
mod select;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::{derive_section_name, format_summary, resolve_input_file, Args};
use crate::config::{load_config_beside_exe, resolve_folderwatch_dir};
use crate::crawljob::{render, RenderOptions};
use crate::filename::sanitize_stem;
use crate::hosts::resolve_alias;
use crate::models::BooksFile;
use crate::output::{atomic_write, deploy_to_watch, write_skipped_report};
use crate::select::select_all;

/// Exit codes per spec §10.
const EXIT_OK: u8 = 0;
const EXIT_SKIPS: u8 = 1;
const EXIT_IO: u8 = 2;
const EXIT_INVALID_INPUT: u8 = 3;

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("error: {e:#}");
            // Distinguish invalid-input vs IO where possible via downcast is overkill here;
            // run() returns specific codes for the common cases and only bubbles unexpected
            // errors to this arm. Treat unexpected as IO.
            ExitCode::from(EXIT_IO)
        }
    }
}

fn run() -> anyhow::Result<u8> {
    let args = Args::parse();

    // Resolve preferred host (validate alias early).
    let preferred = match &args.host {
        Some(h) => match resolve_alias(h) {
            Some(canon) => Some(canon),
            None => {
                eprintln!("error: unknown --host '{h}' (use rg, nf, dd, or a full host name)");
                return Ok(EXIT_INVALID_INPUT);
            }
        },
        None => None,
    };

    // Resolve and read input.
    let input_file = match resolve_input_file(&args.input) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(EXIT_INVALID_INPUT);
        }
    };

    let text = std::fs::read_to_string(&input_file)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", input_file.display()))?;
    let books_file: BooksFile = match serde_json::from_str(&text) {
        Ok(bf) => bf,
        Err(e) => {
            eprintln!("error: {} is not valid books.json: {e}", input_file.display());
            return Ok(EXIT_INVALID_INPUT);
        }
    };

    let section = derive_section_name(books_file.meta.section.as_deref(), &input_file);
    if books_file.meta.section.as_deref().map(str::trim).unwrap_or("").is_empty() {
        eprintln!("warning: _meta.section missing/empty; using '{section}' derived from input file");
    }

    // Select one link per book.
    let sel = select_all(&books_file.books, preferred);

    // Render the crawljob body.
    let opts = RenderOptions {
        package_name: &section,
        download_folder: args.download_folder.as_deref(),
    };
    let body = render(&sel.entries, &opts);

    // Resolve output paths. Artifact base name = sanitized section.
    let file_stem = sanitize_stem(&section);
    let artifact_path: PathBuf = match &args.out {
        Some(p) => p.clone(),
        None => {
            let exe = std::env::current_exe()?;
            let app_dir = exe.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
            app_dir.join(".crawljobs").join(format!("{file_stem}.crawljob"))
        }
    };

    // Write the kept artifact FIRST (work is never lost).
    atomic_write(&artifact_path, &body)
        .map_err(|e| anyhow::anyhow!("failed to write artifact {}: {e}", artifact_path.display()))?;

    // Write skipped report next to the artifact.
    let skipped_path = artifact_path.with_file_name(format!("{file_stem}.skipped.json"));
    let skipped_written = write_skipped_report(
        &skipped_path,
        &section,
        &input_file.to_string_lossy(),
        &sel.skips,
    )?;

    // Print the summary.
    print!("{}", format_summary(&section, books_file.books.len(), &sel));
    println!("Artifact: {}", artifact_path.display());
    if let Some(sp) = &skipped_written {
        println!("Skipped report: {}", sp.display());
    }

    // Deploy unless --no-deploy.
    if !args.no_deploy {
        let cfg = load_config_beside_exe()?;
        let local_app_data = std::env::var("LOCALAPPDATA").ok();
        match resolve_folderwatch_dir(&cfg, local_app_data.as_deref()) {
            Some(watch_dir) => {
                let file_name = format!("{file_stem}.crawljob");
                match deploy_to_watch(&watch_dir, &file_name, &body) {
                    Ok(deployed) => println!("Deployed: {}", deployed.display()),
                    Err(e) => {
                        // Artifact already saved; surface deploy failure as IO error.
                        eprintln!("error: deploy failed: {e}");
                        return Ok(EXIT_IO);
                    }
                }
            }
            None => {
                eprintln!(
                    "error: could not resolve a Folder Watch directory (no config.toml folderwatch_dir \
                     and LOCALAPPDATA unset). Set folderwatch_dir in config.toml or pass --no-deploy."
                );
                return Ok(EXIT_IO);
            }
        }
    }

    // Exit code: 1 if any books were skipped, else 0.
    if sel.skips.is_empty() {
        Ok(EXIT_OK)
    } else {
        Ok(EXIT_SKIPS)
    }
}
```

- [ ] **Step 7: Build and run the whole test suite**

Run: `cargo build`
Expected: compiles with no errors. Fix any unused-import warnings by removing the offending `use` lines (e.g. if `sanitize_stem` or others are flagged, ensure they are actually used; they should be).

Run: `cargo test`
Expected: ALL tests across all modules PASS.

- [ ] **Step 8: Manual smoke test against the small fixture (no deploy)**

Run (PowerShell):

```
cargo run -- tests\fixtures\books_small.json --no-deploy -o .\.crawljobs\smoke.crawljob
```

Expected output (approximately):
```
Computer & Internet: 3 books -> 2 entries written
  by host:   rapidgator 2
  skipped:   1
  filename omitted (JD resolves): 1
  disambiguated filenames: 0
Artifact: ...\.crawljobs\smoke.crawljob
Skipped report: ...\.crawljobs\smoke.skipped.json
```
Exit code should be 1 (one skip). Verify: `echo $LASTEXITCODE` → `1`.

Then inspect the file:
```
Get-Content .\.crawljobs\smoke.crawljob
```
Expected: two `->NEW ENTRY<-` blocks; the Mac Life entry (chosen from rapidgator) has `filename=Mac Life - Nr.07 2026.pdf`; the CI/CD entry has `filename=Ultimate CI-CD for Platform Engineering.epub`; both have `packageName=Computer & Internet`, `autoConfirm=FALSE`, `autoStart=FALSE`, `enabled=TRUE`.

Wait — note: in the small fixture, Mac Life's chosen host under the default chain (rg→dd) is rapidgator (`...pdf.html` → `.pdf`), so `filename` IS present and `filename omitted` should be 0, not 1. **Correct expectation:** `filename omitted (JD resolves): 0`, and `2 entries written` with `skipped: 1` (the nitroflare-only book). The ddownload-no-ext path is only exercised when ddownload is actually chosen, which does not happen here because rapidgator wins. Adjust the manual expectation accordingly and confirm.

- [ ] **Step 9: Clean up smoke artifacts**

Run (PowerShell):

```
Remove-Item .\.crawljobs\smoke.crawljob, .\.crawljobs\smoke.skipped.json -ErrorAction SilentlyContinue
```

- [ ] **Step 10: Commit**

```
git add src/main.rs src/cli.rs
git commit -m "feat: wire CLI, orchestration, summary, and exit codes"
```

---

## Task 10: End-to-end integration test (`tests/e2e.rs`)

A black-box integration test that runs the binary against the real 800-book fixture and asserts on the output file.

**Files:**
- Create: `tests/e2e.rs`

- [ ] **Step 1: Write the failing integration test**

Create `tests/e2e.rs`:

```rust
//! Black-box end-to-end test using the real books.json fixture.

use std::process::Command;

/// Run the compiled binary with args, returning (exit_code, stdout, stderr).
fn run_bin(args: &[&str]) -> (i32, String, String) {
    let exe = env!("CARGO_BIN_EXE_jdpackager");
    let out = Command::new(exe).args(args).output().expect("run jdpackager");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

#[test]
fn converts_real_fixture_no_deploy() {
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("real.crawljob");
    let out_str = out_path.to_string_lossy().to_string();

    let (code, stdout, stderr) = run_bin(&[
        "tests/fixtures/books_real.json",
        "--no-deploy",
        "-o",
        &out_str,
    ]);

    assert!(
        code == 0 || code == 1,
        "expected success/skips exit, got {code}. stderr: {stderr}"
    );
    assert!(out_path.exists(), "output file should exist. stdout: {stdout}");

    let body = std::fs::read_to_string(&out_path).unwrap();

    // Every entry block is well-formed.
    let entry_count = body.matches("->NEW ENTRY<-").count();
    assert!(entry_count > 0, "should have written entries");

    // Each entry has exactly one text= line. Every entry starts with the separator line,
    // so each `text=` is preceded by a newline -> count "\ntext=" occurrences.
    let text_count = body.matches("\ntext=").count();
    assert_eq!(text_count, entry_count, "one text= per entry");

    // The shared package name from _meta.section is present and identical across entries.
    let pkg_count = body.matches("packageName=Computer & Internet").count();
    assert_eq!(pkg_count, entry_count, "shared packageName on every entry");

    // Sanity: summary mentions the section and 800 books.
    assert!(stdout.contains("Computer & Internet"), "summary should name the section");
    assert!(stdout.contains("800 books"), "summary should report 800 books, got: {stdout}");
}

#[test]
fn host_nf_changes_selection_counts() {
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("nf.crawljob");
    let out_str = out_path.to_string_lossy().to_string();

    let (code, stdout, _stderr) = run_bin(&[
        "tests/fixtures/books_real.json",
        "--host",
        "nf",
        "--no-deploy",
        "-o",
        &out_str,
    ]);
    assert!(code == 0 || code == 1);
    // With nf preferred, nitroflare should now be chosen for at least some books
    // (the real data has many nitroflare links), so "nitroflare N" with N>0 appears.
    assert!(
        stdout.contains("nitroflare ") && !stdout.contains("nitroflare 0"),
        "with --host nf, nitroflare count should be > 0. stdout: {stdout}"
    );
}
```

- [ ] **Step 2: Run the integration tests**

Run: `cargo test --test e2e`
Expected: both tests PASS. If `host_nf_changes_selection_counts` fails because the real data happens to have rapidgator on every nitroflare book (so nitroflare is never chosen)... that cannot happen: `--host nf` puts nitroflare FIRST, so any book with a nitroflare link picks it. The only way nitroflare count is 0 is if no book has a nitroflare link, which is false for this fixture. If it still fails, inspect `stdout` to confirm the host tallies and adjust the assertion to `!stdout.contains("nitroflare 0")` only.

- [ ] **Step 3: Run the FULL suite once more**

Run: `cargo test`
Expected: every unit + integration test PASSES.

- [ ] **Step 4: Commit**

```
git add tests/e2e.rs
git commit -m "test: add end-to-end integration tests against real fixture"
```

---

## Task 11: Documentation & config template

**Files:**
- Create: `README.md`
- Create: `config.toml.example`

- [ ] **Step 1: Write `config.toml.example`**

```toml
# Copy this file to `config.toml` (next to the jdpackager executable) and edit the path.
# JDownloader Folder Watch directory. The generated .crawljob is copied here to auto-import.
# If omitted, defaults to %LOCALAPPDATA%\JDownloader 2.0\folderwatch
folderwatch_dir = "C:\\Users\\YOURNAME\\AppData\\Local\\JDownloader 2.0\\folderwatch"
```

- [ ] **Step 2: Write `README.md`**

````markdown
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
- `--host <h>` — preferred host at the head of the selection chain. Aliases: `rg`
  (rapidgator), `nf` (nitroflare), `dd` (ddownload). Default chain: `rapidgator -> ddownload -> skip`.
  `--host nf` -> `nitroflare -> rapidgator -> ddownload -> skip`.
- `-o <path>` — override the kept-artifact path. Default: `<exe dir>\.crawljobs\<section>.crawljob`.
- `--no-deploy` — write only the local artifact; don't copy into the Folder Watch folder.
- `--download-folder <path>` — set `downloadFolder=` in every entry (default: JDownloader's default dir).

## Configuration

`jdpackager` copies the generated `.crawljob` into JDownloader's Folder Watch folder so it
auto-imports. Set the folder in `config.toml` next to the executable (see `config.toml.example`).
Defaults to `%LOCALAPPDATA%\JDownloader 2.0\folderwatch`.

**JDownloader must have the Folder Watch extension enabled** for auto-import to work
(Settings -> Extensions -> Folder Watch).

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

## Development

```
cargo test          # run all unit + integration tests
cargo run -- tests/fixtures/books_small.json --no-deploy -o .\.crawljobs\smoke.crawljob
```
````

- [ ] **Step 3: Commit**

```
git add README.md config.toml.example
git commit -m "docs: add README and config.toml example"
```

---

## Task 12: Live JDownloader verification (manual — resolves spec §13 / V1)

This is a **manual** verification step requiring a running JDownloader with the Folder Watch
extension enabled. It validates assumptions the unit tests cannot (real import behavior).

- [ ] **Step 1: Ensure config points at the real Folder Watch folder**

Copy `config.toml.example` to `config.toml` next to the built binary (or use `cargo run` from the
project root, which resolves the exe under `target\debug\`; place `config.toml` there, or rely on
the `%LOCALAPPDATA%` default). Confirm JDownloader's Folder Watch is enabled and note its watched
folder (Settings -> Folder Watch -> Folders).

- [ ] **Step 2: Generate and deploy from the real fixture**

Run (PowerShell):

```
cargo run -- tests\fixtures\books_real.json
```

Expected: summary prints, artifact written under `.crawljobs\`, and `Deployed: ...folderwatch\Computer & Internet.crawljob` printed. Exit code 0 or 1.

- [ ] **Step 3: Verify in JDownloader**

In JDownloader's LinkGrabber, confirm:
1. The links import (give it a few seconds; the file moves to `folderwatch\added\`).
2. All entries appear under **one** package named "Computer & Internet" (validates the
   shared-`packageName` grouping — spec §5 V1).
3. Filenames shown match the clean Titles (e.g. "Ultimate CI-CD for Platform Engineering.epub")
   for rapidgator-sourced entries.
4. Nothing auto-starts downloading (collect-only).

- [ ] **Step 4: Record the outcome**

If all four hold, append a short note to the spec under §13 marking V1 resolved (date + result).
If the parser needs blank lines around `->NEW ENTRY<-`, or the package does not group, update
`src/crawljob.rs` `render` accordingly, re-run `cargo test crawljob::` (update the golden tests to
match), and re-verify. Commit any fix:

```
git add src/crawljob.rs docs/superpowers/specs/2026-06-03-jdownloader-packager-design.md
git commit -m "fix: adjust crawljob formatting per live JDownloader verification"
```

---

## Done

When all tasks are complete:
- `cargo test` is green (unit + integration).
- The CLI converts the real `books.json`, writes a kept artifact, and deploys to Folder Watch.
- Live JDownloader import is verified (one package, clean filenames, collect-only).
- README + config template are in place.

Final full-suite check and a summary commit if anything is uncommitted:

```
cargo test
git status   # should be clean
```
