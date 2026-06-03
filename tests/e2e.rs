//! Black-box end-to-end tests using the real and small books.json fixtures.

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

    // Contrast: under the DEFAULT chain (rapidgator -> ddownload, nitroflare excluded),
    // nitroflare must NOT be selected for any book. This proves --host nf actually
    // changed selection rather than nitroflare being chosen anyway.
    let default_out = tmp.path().join("default.crawljob");
    let default_str = default_out.to_string_lossy().to_string();
    let (dcode, dstdout, _dstderr) = run_bin(&[
        "tests/fixtures/books_real.json",
        "--no-deploy",
        "-o",
        &default_str,
    ]);
    assert!(dcode == 0 || dcode == 1);
    assert!(
        !dstdout.contains("nitroflare "),
        "default chain must never select nitroflare. stdout: {dstdout}"
    );
}

#[test]
fn small_fixture_skips_yield_exit_1_and_report() {
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("small.crawljob");
    let out_str = out_path.to_string_lossy().to_string();

    let (code, stdout, _stderr) = run_bin(&[
        "tests/fixtures/books_small.json",
        "--no-deploy",
        "-o",
        &out_str,
    ]);
    // Small fixture has one nitroflare-only book -> skipped -> exit 1.
    assert_eq!(code, 1, "one skip should yield exit 1. stdout: {stdout}");
    assert!(out_path.exists(), "artifact should exist even with skips");
    // The sidecar report tracks the -o basename.
    let report = tmp.path().join("small.skipped.json");
    assert!(report.exists(), "skipped report should exist next to artifact");
    let report_body = std::fs::read_to_string(&report).unwrap();
    assert!(report_body.contains("Nitroflare Only Book"));
    assert!(report_body.contains("no link on rapidgator/ddownload"));
}

#[test]
fn bad_host_exits_3() {
    let tmp = tempfile::tempdir().unwrap();
    let out_str = tmp.path().join("x.crawljob").to_string_lossy().to_string();
    let (code, _stdout, stderr) = run_bin(&[
        "tests/fixtures/books_small.json",
        "--host",
        "mega",
        "--no-deploy",
        "-o",
        &out_str,
    ]);
    assert_eq!(code, 3, "unknown --host should exit 3. stderr: {stderr}");
    assert!(stderr.contains("unknown --host"));
}

#[test]
fn missing_input_exits_3() {
    let (code, _stdout, stderr) = run_bin(&["tests/fixtures/does_not_exist.json", "--no-deploy"]);
    assert_eq!(code, 3, "missing input should exit 3. stderr: {stderr}");
}

#[test]
fn invalid_json_exits_3() {
    let tmp = tempfile::tempdir().unwrap();
    let bad = tmp.path().join("bad.json");
    std::fs::write(&bad, "{ this is not valid json ").unwrap();
    let bad_str = bad.to_string_lossy().to_string();
    let out_str = tmp.path().join("x.crawljob").to_string_lossy().to_string();
    let (code, _stdout, stderr) = run_bin(&[&bad_str, "--no-deploy", "-o", &out_str]);
    assert_eq!(code, 3, "invalid JSON should exit 3. stderr: {stderr}");
}

#[test]
fn artifact_written_with_no_deploy_does_not_require_folderwatch() {
    // --no-deploy must succeed and write the artifact regardless of any folderwatch config.
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("nodeploy.crawljob");
    let out_str = out_path.to_string_lossy().to_string();
    let (code, stdout, _stderr) = run_bin(&[
        "tests/fixtures/books_small.json",
        "--no-deploy",
        "-o",
        &out_str,
    ]);
    assert!(code == 0 || code == 1, "no-deploy run should not be an IO error. stdout: {stdout}");
    assert!(out_path.exists());
    assert!(!stdout.contains("Deployed:"), "no-deploy must not deploy");
}
