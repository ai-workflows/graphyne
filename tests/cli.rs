use std::process::Command;

fn binary_path() -> String {
    std::env::var("CARGO_BIN_EXE_graphyne").expect("binary path should be available in integration tests")
}

#[test]
fn await_mode_runs_example_program() {
    let output = Command::new(binary_path())
        .args(["await", "-i", "examples/intermediate/test_compiled.json"])
        .output()
        .expect("failed to run graphyne await");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("out | 0: [20,40,60]"));
    assert!(stdout.contains("out | 1: \"World\""));
    assert!(stdout.contains("out | 2: 60"));
}

#[test]
fn stream_mode_runs_example_program() {
    let output = Command::new(binary_path())
        .args(["stream", "-i", "examples/intermediate/test_compiled.json"])
        .output()
        .expect("failed to run graphyne stream");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("out | 0: [20,40,60]"));
    assert!(stdout.contains("out | 1: \"World\""));
    assert!(stdout.contains("out | 2: 60"));
}

#[test]
fn invalid_input_path_reports_error_on_stderr() {
    let output = Command::new(binary_path())
        .args(["await", "-i", "examples/intermediate/does_not_exist.json"])
        .output()
        .expect("failed to run graphyne await with invalid path");

    assert!(output.stdout.is_empty(), "unexpected stdout: {}", String::from_utf8_lossy(&output.stdout));

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Error loading intermediate program"));
}

#[test]
fn verbose_mode_writes_info_to_stderr_without_polluting_stdout() {
    let output = Command::new(binary_path())
        .args(["await", "-i", "examples/intermediate/test_compiled.json", "--verbose"])
        .output()
        .expect("failed to run graphyne await in verbose mode");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("out | 0: [20,40,60]"));
    assert!(stdout.contains("out | 1: \"World\""));
    assert!(stdout.contains("out | 2: 60"));
    assert!(!stdout.contains("info:"));

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("info: mode=await"));
    assert!(stderr.contains("info: received 3 outputs"));
}

#[test]
fn invalid_program_reports_bind_error_without_panicking() {
    let output = Command::new(binary_path())
        .args(["await", "-i", "examples/intermediate/double_list.json"])
        .output()
        .expect("failed to run graphyne await with invalid program");

    assert!(output.status.success(), "expected graceful exit, got status {:?}", output.status.code());
    assert!(output.stdout.is_empty(), "unexpected stdout: {}", String::from_utf8_lossy(&output.stdout));

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Error binding program"));
    assert!(!stderr.contains("panicked at"));
}
