//! Load config.toml (next to the executable) and resolve the Folder Watch directory.

use std::fs;
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
