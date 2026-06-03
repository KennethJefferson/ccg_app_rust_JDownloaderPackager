//! CLI argument definitions and pure helpers (input resolution, section derivation, summary).

use std::path::{Path, PathBuf};

use clap::Parser;

use crate::select::Selection;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::select::Entry;

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
}
