use std::process::Command;

/// Run aethel CLI via cargo run --release for integration testing.
fn run_aethel(args: &[&str]) -> (bool, String) {
    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "-p", "aethel-cli",
            "--",
        ])
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR")) // workspace root
        .output()
        .expect("failed to run aethel-cli");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    // Filter out cargo build noise — only get CLI output
    let combined = format!("{}{}", stdout, stderr);
    // Strip cargo build messages
    let clean: Vec<&str> = combined.lines()
        .filter(|l| !l.contains("Compiling") && !l.contains("Finished") && !l.contains("warning:")
            && !l.contains("Running") && !l.contains("help:") && !l.contains("="))
        .collect();
    (output.status.success(), clean.join("\n"))
}

#[test]
fn test_check_valid_passes() {
    let (ok, output) = run_aethel(&["check", "examples/refund/valid_verified.aet"]);
    assert!(ok, "valid program should pass check\n{}", output);
    assert!(output.contains("type checks"), "should say type checks ok");
}

#[test]
fn test_check_invalid_fails() {
    let (ok, output) = run_aethel(&["check", "examples/refund/invalid_unverified.aet"]);
    assert!(!ok, "invalid program should fail check\n{}", output);
    assert!(output.contains("AE-EPISTEMIC-001"), "should emit epistemic error");
}

#[test]
fn test_run_valid_no_violations() {
    let (ok, output) = run_aethel(&["run", "examples/refund/valid_verified.aet"]);
    assert!(ok, "valid program should run without error\n{}", output);
    assert!(output.contains("No policy violations"), "should report no violations");
}

#[test]
fn test_run_invalid_typecheck_fails() {
    let (ok, output) = run_aethel(&["run", "examples/refund/invalid_unverified.aet"]);
    assert!(!ok, "invalid program run should fail at typecheck\n{}", output);
}

#[test]
fn test_run_trace_shows_effect() {
    let (ok, output) = run_aethel(&["run", "examples/refund/valid_verified.aet", "--trace"]);
    assert!(ok, "trace mode should run\n{}", output);
    assert!(output.contains("Effect Trace"), "should show effect trace section");
}

#[test]
fn test_emit_ir_produces_json() {
    let (ok, output) = run_aethel(&["emit-ir", "examples/refund/valid_verified.aet"]);
    assert!(ok, "emit-ir should succeed\n{}", output);
    assert!(output.contains("ir_version"), "output should contain ir_version");
}

#[test]
fn test_simple_fn_pipeline() {
    let (_file, path) = tempfile::NamedTempFile::new()
        .unwrap()
        .path()
        .to_str()
        .unwrap()
        .to_string();
    // Create a simple test file
    let test_path = std::path::Path::new("examples/_test_simple.aet");
    std::fs::write(test_path, "fn main() { let x = 42; }").unwrap();

    let (ok, output) = run_aethel(&["check", "examples/_test_simple.aet"]);
    assert!(ok, "simple fn should pass check\n{}", output);

    let (ok, output) = run_aethel(&["run", "examples/_test_simple.aet"]);
    assert!(ok, "simple fn should run\n{}", output);

    std::fs::remove_file(test_path).ok();
}
