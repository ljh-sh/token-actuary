//! Integration tests for the token-actuary CLI.

use std::io::Write;
use std::process::{Command, Stdio};

fn run(args: &[&str], stdin: Option<&str>) -> (bool, String, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_token-actuary"));
    for arg in args {
        cmd.arg(arg);
    }
    if stdin.is_some() {
        cmd.stdin(Stdio::piped());
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn token-actuary");
    if let Some(input) = stdin {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .expect("failed to write stdin");
    }
    let output = child.wait_with_output().expect("failed to read output");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn count_embedded_gpt4o() {
    let (ok, stdout, stderr) = run(&["count"], Some("hello world\n"));
    assert!(ok, "stderr: {}", stderr);
    assert_eq!(stdout.trim(), "3");
}

#[test]
fn count_with_model_flag() {
    let (ok, stdout, stderr) = run(&["count", "--model", "gpt-4"], Some("hello world\n"));
    assert!(ok, "stderr: {}", stderr);
    assert_eq!(stdout.trim(), "3");
}

#[test]
fn count_with_tokenizer_file() {
    let tokenizer = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/minimal-tokenizer.json");
    let (ok, stdout, stderr) = run(&["count", "--tokenizer", tokenizer], Some("hello world\n"));
    assert!(ok, "stderr: {}", stderr);
    assert_eq!(stdout.trim(), "2");
}

#[test]
fn encode_decode_roundtrip_comma() {
    let (ok, stdout, stderr) = run(&["encode"], Some("hello world\n"));
    assert!(ok, "encode stderr: {}", stderr);
    let ids = stdout.trim();

    let (ok, stdout, stderr) = run(&["decode", ids], None);
    assert!(ok, "decode stderr: {}", stderr);
    assert_eq!(stdout.trim(), "hello world");
}

#[test]
fn encode_decode_roundtrip_pipe() {
    let (ok, encoded, stderr) = run(&["encode"], Some("hello world\n"));
    assert!(ok, "encode stderr: {}", stderr);

    let (ok, decoded, stderr) = run(&["decode"], Some(&encoded));
    assert!(ok, "decode stderr: {}", stderr);
    assert_eq!(decoded.trim(), "hello world");
}

#[test]
fn encode_decode_custom_separator() {
    let (ok, encoded, stderr) = run(&["encode", "-s", " | "], Some("hello world\n"));
    assert!(ok, "encode stderr: {}", stderr);
    assert!(encoded.contains(" | "), "custom sep not in output: {}", encoded);

    let (ok, decoded, stderr) = run(&["decode", "-s", " | "], Some(&encoded));
    assert!(ok, "decode stderr: {}", stderr);
    assert_eq!(decoded.trim(), "hello world");
}

#[test]
fn decode_from_positional_args() {
    let (ok, stdout, stderr) = run(&["decode", "24912,2375,198"], None);
    assert!(ok, "stderr: {}", stderr);
    assert_eq!(stdout.trim(), "hello world");
}

#[test]
fn decode_from_multiple_positional_args() {
    let (ok, stdout, stderr) = run(&["decode", "24912", "2375", "198"], None);
    assert!(ok, "stderr: {}", stderr);
    assert_eq!(stdout.trim(), "hello world");
}

#[test]
fn audit_json_output() {
    let (ok, stdout, stderr) = run(
        &[
            "audit",
            "--redact",
            "secret,password",
            "--replace",
            "[REDACTED],[REDACTED]",
            "--max-tokens",
            "100",
            "--format",
            "json",
        ],
        Some("my secret password is here"),
    );
    assert!(ok, "stderr: {}", stderr);
    assert!(stdout.contains("\"redaction_hits\": 2"), "stdout: {}", stdout);
    assert!(stdout.contains("[REDACTED]"), "stdout: {}", stdout);
}

#[test]
fn audit_truncates_to_max_tokens() {
    let (ok, stdout, stderr) = run(
        &[
            "audit",
            "--max-tokens",
            "2",
        ],
        Some("hello world here"),
    );
    assert!(ok, "stderr: {}", stderr);
    assert!(stdout.contains("truncated:"), "stdout: {}", stdout);
}

#[test]
fn compare_outputs_tsv_header() {
    let (ok, stdout, stderr) = run(&["compare"], Some("hello world\n"));
    assert!(ok, "stderr: {}", stderr);
    assert!(stdout.starts_with("input\tmodel\ttokens"), "stdout: {}", stdout);
    assert!(stdout.contains("gpt-4o"), "stdout: {}", stdout);
}

#[test]
fn help_shows_subcommands() {
    let (ok, stdout, stderr) = run(&["--help"], None);
    assert!(ok, "stderr: {}", stderr);
    assert!(stdout.contains("count"));
    assert!(stdout.contains("audit"));
    assert!(stdout.contains("encode"));
    assert!(stdout.contains("decode"));
    assert!(stdout.contains("download"));
    assert!(stdout.contains("compare"));
}

#[test]
fn download_help_shows_flags() {
    let (ok, stdout, stderr) = run(&["download", "--help"], None);
    assert!(ok, "stderr: {}", stderr);
    assert!(stdout.contains("--china"), "stdout: {}", stdout);
    assert!(stdout.contains("--recommend"), "stdout: {}", stdout);
}

#[test]
fn invalid_tokenizer_file_fails() {
    let (ok, _stdout, _stderr) = run(&["count", "--tokenizer", "/does/not/exist.json"], Some("hi"));
    assert!(!ok);
}
