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
fn double_list_example_runs_successfully() {
    let output = Command::new(binary_path())
        .args(["await", "-i", "examples/intermediate/double_list.json"])
        .output()
        .expect("failed to run graphyne await for double_list example");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("out | 0: [20,40,60]"));
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
    let invalid_program_path = std::env::temp_dir().join("graphyne-invalid-bind.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "main": {
      "graph": {
        "values": ["missing_symbol", "result"],
        "ops": [["Get", ["missing_symbol", "result"], ["result"]]],
        "input_vals": [],
        "output_vals": ["result"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with invalid program");

    assert!(output.status.success(), "unexpected exit status {:?}", output.status.code());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Error binding program") || stderr.contains("Error starting program"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn runtime_operator_error_reports_cleanly_without_abort() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-runtime-error.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "types": {
    "Person": [["age", "int"]]
  },
  "functions": {
    "main": {
      "graph": {
        "values": ["person_type", ["bad_age", "not-an-int"], "person"],
        "ops": [
          ["Static", ["Person"], ["person_type"]],
          ["Init", ["person_type", "bad_age"], ["person"]]
        ],
        "input_vals": [],
        "output_vals": ["person"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with runtime-error program");

    assert!(output.status.success(), "unexpected exit status {:?}", output.status.code());
    assert!(output.stdout.is_empty(), "unexpected stdout: {}", String::from_utf8_lossy(&output.stdout));

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Error starting program") || stderr.contains("Runtime error"));
    assert!(stderr.contains("Cannot initialize object of type Person"));
    assert!(!stderr.contains("panicked at"));
    assert!(!stderr.contains("SIGABRT"));
}
