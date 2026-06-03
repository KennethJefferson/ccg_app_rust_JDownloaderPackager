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

/// Exit codes per spec.
const EXIT_OK: u8 = 0;
const EXIT_SKIPS: u8 = 1;
const EXIT_IO: u8 = 2;
const EXIT_INVALID_INPUT: u8 = 3;

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("error: {e:#}");
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

    // Write skipped report next to the artifact, named after the artifact's own stem
    // (so `-o custom.crawljob` yields `custom.skipped.json`, not a section-named sidecar).
    let report_stem = artifact_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file_stem.clone());
    let skipped_path = artifact_path.with_file_name(format!("{report_stem}.skipped.json"));
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
