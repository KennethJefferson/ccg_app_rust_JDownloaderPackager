//! Render selected entries into JDownloader .crawljob properties text.

use crate::select::Entry;
use std::fmt::Write;

/// Options that apply to every entry in the file.
pub struct RenderOptions<'a> {
    pub package_name: &'a str,
    pub download_folder: Option<&'a str>,
}

/// Render all entries into the full .crawljob file body (UTF-8, LF line endings).
/// Each entry is preceded by a `->NEW ENTRY<-` line. Returns an empty string when
/// there are no entries.
pub fn render(entries: &[Entry], opts: &RenderOptions) -> String {
    let mut out = String::new();
    for e in entries {
        out.push_str("->NEW ENTRY<-\n");
        let _ = writeln!(out, "text={}", e.url);
        if let Some(name) = &e.filename {
            let _ = writeln!(out, "filename={}", name);
        }
        let _ = writeln!(out, "packageName={}", opts.package_name);
        if let Some(df) = opts.download_folder {
            let _ = writeln!(out, "downloadFolder={}", df);
        }
        out.push_str("autoConfirm=FALSE\nautoStart=FALSE\nenabled=TRUE\n");
    }
    out
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

    #[test]
    fn renders_no_filename_with_download_folder() {
        let entries = vec![entry("https://ddownload.com/abc123", None, "ddownload")];
        let opts = RenderOptions { package_name: "Sec", download_folder: Some("D:\\Books") };
        let out = render(&entries, &opts);
        let expected = "\
->NEW ENTRY<-
text=https://ddownload.com/abc123
packageName=Sec
downloadFolder=D:\\Books
autoConfirm=FALSE
autoStart=FALSE
enabled=TRUE
";
        assert_eq!(out, expected);
    }
}
