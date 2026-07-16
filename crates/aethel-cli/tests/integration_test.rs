use std::path::Path;
use std::process::Command;

fn run_aethel(args: &[&str]) -> (bool, String) {
    let md = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ws = md.parent().unwrap().parent().unwrap();
    let bin_name = if cfg!(target_os = "windows") { "aethel-cli.exe" } else { "aethel-cli" };
    let bin = ws.join("target").join("release").join(bin_name);
    assert!(bin.exists(), "Binary not found at {:?}", bin);
    let out = Command::new(&bin).args(args).current_dir(ws).output().unwrap();
    let s = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    (out.status.success(), s)
}

// ── Check tests ──
#[test] fn test_check_valid() {
    let (ok, o) = run_aethel(&["check", "examples/refund/valid_verified.aet"]);
    assert!(ok && o.contains("type checks"), "valid should pass");
}
#[test] fn test_check_invalid() {
    let (ok, o) = run_aethel(&["check", "examples/refund/invalid_unverified.aet"]);
    assert!(!ok && o.contains("AE-EPISTEMIC-001"), "invalid should fail: {o}");
}
#[test] fn test_check_full_pipeline() {
    let (ok, o) = run_aethel(&["check", "examples/full_pipeline.aet"]);
    assert!(ok && o.contains("type checks"), "pipeline should pass: {o}");
}

// ── Run tests ──
#[test] fn test_run_valid() {
    let (ok, o) = run_aethel(&["run", "examples/refund/valid_verified.aet"]);
    assert!(ok && o.contains("No policy violations"), "should pass: {o}");
}
#[test] fn test_run_valid_trace() {
    let (ok, o) = run_aethel(&["run", "examples/refund/valid_verified.aet", "--trace"]);
    assert!(ok && o.contains("Effect Trace"), "trace: {o}");
}
#[test] fn test_run_full_pipeline() {
    let (ok, o) = run_aethel(&["run", "examples/full_pipeline.aet"]);
    assert!(ok && o.contains("No policy violations"), "pipeline: {o}");
}
#[test] fn test_run_full_trace() {
    let (ok, o) = run_aethel(&["run", "examples/full_pipeline.aet", "--trace"]);
    assert!(ok && o.contains("Effect Trace"), "trace: {o}");
    assert!(o.contains("log_action") && o.contains("execute"), "effects: {o}");
}
#[test] fn test_run_invalid_fails() {
    let (ok, _) = run_aethel(&["run", "examples/refund/invalid_unverified.aet"]);
    assert!(!ok, "invalid should fail");
}

// ── Emit IR ──
#[test] fn test_emit_ir() {
    let (ok, o) = run_aethel(&["emit-ir", "examples/refund/valid_verified.aet"]);
    assert!(ok && o.contains("ir_version"), "emit-ir: {o}");
}
