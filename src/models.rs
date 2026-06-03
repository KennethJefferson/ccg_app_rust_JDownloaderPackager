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
