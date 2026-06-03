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
    use crate::hosts::NITROFLARE;
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

    #[test]
    fn preferred_host_selects_nitroflare_over_default() {
        // Book has both nitroflare and rapidgator. With preferred = nitroflare,
        // the nitroflare link must be chosen (default chain would pick rapidgator).
        let books = vec![book(
            "Delta",
            &[
                ("rapidgator", "https://rapidgator.net/file/d/Delta.epub.html"),
                ("nitroflare", "https://nitroflare.com/v/D/Delta.sanet.st.epub"),
            ],
        )];
        let sel = select_all(&books, Some(NITROFLARE));
        assert_eq!(sel.entries.len(), 1);
        assert_eq!(sel.entries[0].host, "nitroflare");
        assert_eq!(sel.entries[0].url, "https://nitroflare.com/v/D/Delta.sanet.st.epub");
        assert_eq!(sel.entries[0].filename.as_deref(), Some("Delta.epub"));
        assert_eq!(*sel.host_counts.get("nitroflare").unwrap(), 1);
    }

    #[test]
    fn host_counts_accumulates_multiple_hosts() {
        // Two books resolved from different hosts under the default chain:
        // one rapidgator, one ddownload-only (no rapidgator present).
        let books = vec![
            book("Echo", &[("rapidgator", "https://rapidgator.net/file/e/Echo.pdf.html")]),
            book("Foxtrot", &[("ddownload", "https://ddownload.com/file/f/Foxtrot.pdf")]),
        ];
        let sel = select_all(&books, None);
        assert_eq!(sel.entries.len(), 2);
        assert_eq!(*sel.host_counts.get("rapidgator").unwrap(), 1);
        assert_eq!(*sel.host_counts.get("ddownload").unwrap(), 1);
        assert_eq!(sel.host_counts.len(), 2);
    }
}
