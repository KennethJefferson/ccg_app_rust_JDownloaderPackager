//! Pure filename logic: extension derivation, Title sanitization, collision disambiguation.

use std::collections::HashSet;

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
    let trimmed = collapsed.trim_end_matches(['.', ' ']).to_string();

    // 4. Fall back if empty.
    let mut result = if trimmed.is_empty() { "untitled".to_string() } else { trimmed };

    // 5. Truncate to cap on a char boundary.
    if result.chars().count() > MAX_STEM_CHARS {
        result = result.chars().take(MAX_STEM_CHARS).collect();
        // Re-trim trailing space/dot that truncation may have exposed.
        result = result.trim_end_matches(['.', ' ']).to_string();
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

/// Build the full `filename=` value `<sanitized stem>.<ext>`.
/// Returns None when there is no extension (caller then omits the filename= key).
pub fn build_filename(title: &str, ext: Option<&str>) -> Option<String> {
    let ext = ext?;
    Some(format!("{}.{}", sanitize_stem(title), ext))
}

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
        assert_eq!(sanitize_stem("***"), "untitled");
        assert_eq!(sanitize_stem("   "), "untitled");
    }

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

    #[test]
    fn disambiguator_appends_counter_for_duplicates() {
        let mut d = Disambiguator::new();
        assert_eq!(d.unique("Book.epub"), "Book.epub");
        assert_eq!(d.unique("Book.epub"), "Book (2).epub");
        assert_eq!(d.unique("Book.epub"), "Book (3).epub");
        assert_eq!(d.unique("Other.pdf"), "Other.pdf");
    }

    #[test]
    fn disambiguator_is_case_insensitive() {
        let mut d = Disambiguator::new();
        assert_eq!(d.unique("Book.epub"), "Book.epub");
        assert_eq!(d.unique("book.EPUB"), "book (2).EPUB");
    }
}
