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
    if !watch_dir.is_dir() {
        return Err(anyhow::anyhow!(
            "JDownloader Folder Watch directory does not exist (or is not a directory): {}\n\
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
        // No leftover temp file in the watch dir (the rename consumed it).
        let leftover: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftover.is_empty(), "deploy should leave no .tmp in watch dir");
    }

    #[test]
    fn deploy_errors_when_watch_dir_is_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("not_a_dir");
        fs::write(&file_path, "i am a file").unwrap();
        let err = deploy_to_watch(&file_path, "x.crawljob", "BODY").unwrap_err();
        assert!(err.to_string().contains("not a directory"));
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
