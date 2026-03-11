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
    assert!(!stderr.contains("\u{1b}[31m"));
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

    assert!(!output.status.success(), "expected non-zero exit status for bind error");

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Error binding program") || stderr.contains("Error starting program"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn runtime_type_errors_are_reported_without_aborting() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-runtime-type-error.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "types": {
    "Person": [
      ["name", "str"],
      ["age", "int"]
    ]
  },
  "functions": {
    "main": {
      "graph": {
        "values": [
          "Person",
          ["name", "Ada"],
          ["age", "thirty six"],
          "person"
        ],
        "ops": [
          ["Static", ["Person"], ["Person"]],
          ["Init", ["Person", "name", "age"], ["person"]]
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
        .expect("failed to run graphyne await with runtime type error");

    assert!(!output.status.success(), "expected non-zero exit status for runtime error");
    assert!(output.stdout.is_empty(), "unexpected stdout: {}", String::from_utf8_lossy(&output.stdout));

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Cannot initialize object of type Person"));
    assert!(!stderr.contains("Rayon: detected unexpected panic"));
    assert!(!stderr.contains("panicked at"));
    assert!(!stderr.contains("core dumped"));
}

#[test]
fn stream_runtime_operator_error_reports_cleanly_without_hanging() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-stream-runtime-error.json");
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
        .args(["stream", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne stream with runtime-error program");

    assert!(!output.status.success(), "expected non-zero exit status for runtime error");
    assert!(output.stdout.is_empty(), "unexpected stdout: {}", String::from_utf8_lossy(&output.stdout));

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Runtime error") || stderr.contains("Error starting program"));
    assert!(stderr.contains("Cannot initialize object of type Person"));
    assert!(!stderr.contains("panicked at"));
}

fn write_import_program(import_path: &str, file_name: &str) -> std::path::PathBuf {
    let import_program_path = std::env::temp_dir().join(file_name);
    std::fs::write(
        &import_program_path,
        format!(
            r#"{{
  "collections": {{
    "lib": {{
      "constants": {{
        "two": 2
      }},
      "functions": {{
        "double": {{
          "graph": {{
            "values": ["two", "num", "result"],
            "ops": [
              ["Static", ["two"], ["two"]],
              ["Mul", ["num", "two"], ["result"]]
            ],
            "input_vals": ["num"],
            "output_vals": ["result"]
          }}
        }}
      }}
    }}
  }},
  "imports": {{
    "double": {import_path}
  }},
  "functions": {{
    "main": {{
      "graph": {{
        "values": ["double", ["value", 21], "result"],
        "ops": [
          ["Static", ["double"], ["double"]],
          ["Call", ["double", "value"], ["result"]]
        ],
        "input_vals": [],
        "output_vals": ["result"]
      }}
    }}
  }}
}}"#
        ),
    )
    .unwrap();
    import_program_path
}

#[test]
fn imports_example_using_root_relative_paths_runs_successfully() {
    let import_program_path = write_import_program(r#"["lib", "double"]"#, "graphyne-imports-root-relative.json");

    let output = Command::new(binary_path())
        .args(["await", "-i", import_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with imports program");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("out | 0: 42"));
}

#[test]
fn imports_example_accepts_user_visible_root_symbol_in_import_path() {
    let import_program_path = write_import_program(r#"["root", "lib", "double"]"#, "graphyne-imports-rooted.json");

    let output = Command::new(binary_path())
        .args(["await", "-i", import_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with rooted imports program");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("out | 0: 42"));
}

#[test]
fn object_person_example_runs_successfully() {
    let output = Command::new(binary_path())
        .args(["await", "-i", "examples/intermediate/object_person.json"])
        .output()
        .expect("failed to run graphyne await for object_person example");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"name\":\"Ada\""));
    assert!(stdout.contains("\"age\":37"));
    assert!(stdout.contains("out | 1: 37"));
}

#[test]
fn object_person_example_uses_map_form_type_definition() {
    let output = Command::new(binary_path())
        .args(["await", "-i", "examples/intermediate/object_person.json"])
        .output()
        .expect("failed to run graphyne await for object_person example");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"name\":\"Person\""));
}

#[test]
fn object_set_wrong_field_type_reports_clean_runtime_error() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-object-wrong-field-type.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "types": {
    "Person": {
      "name": "str",
      "age": "int"
    }
  },
  "functions": {
    "main": {
      "graph": {
        "values": [
          "Person",
          ["name", "Ada"],
          ["age", 36],
          "person",
          ["age_key", "age"],
          ["wrong_age", "thirty seven"],
          "updated_person"
        ],
        "ops": [
          ["Static", ["Person"], ["Person"]],
          ["Init", ["Person", "name", "age"], ["person"]],
          ["Set", ["person", "age_key", "wrong_age"], ["updated_person"]]
        ],
        "input_vals": [],
        "output_vals": ["updated_person"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with wrong object field type");

    assert!(!output.status.success(), "expected non-zero exit status for runtime error");
    assert!(output.stdout.is_empty(), "unexpected stdout: {}", String::from_utf8_lossy(&output.stdout));

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Cannot set field age of type Integer to value of type String"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn object_set_missing_field_reports_clean_runtime_error() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-object-missing-field.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "types": {
    "Person": [
      ["name", "str"],
      ["age", "int"]
    ]
  },
  "functions": {
    "main": {
      "graph": {
        "values": [
          "Person",
          ["name", "Ada"],
          ["age", 36],
          "person",
          ["height_key", "height"],
          ["new_height", 170],
          "updated_person"
        ],
        "ops": [
          ["Static", ["Person"], ["Person"]],
          ["Init", ["Person", "name", "age"], ["person"]],
          ["Set", ["person", "height_key", "new_height"], ["updated_person"]]
        ],
        "input_vals": [],
        "output_vals": ["updated_person"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with missing object field");

    assert!(!output.status.success(), "expected non-zero exit status for runtime error");
    assert!(output.stdout.is_empty(), "unexpected stdout: {}", String::from_utf8_lossy(&output.stdout));

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Key height not found"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn imported_object_sum_example_runs_successfully() {
    let output = Command::new(binary_path())
        .args(["await", "-i", "examples/intermediate/imported_object_sum.json"])
        .output()
        .expect("failed to run graphyne await for imported_object_sum example");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("out | 0: 31"));
}

#[test]
fn collections_of_objects_example_runs_successfully() {
    let output = Command::new(binary_path())
        .args(["await", "-i", "examples/intermediate/collections_of_objects.json"])
        .output()
        .expect("failed to run graphyne await for collections_of_objects example");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("out | 0: \"Grace\""));
    assert!(stdout.contains("out | 1: \"Ada\""));
}

#[test]
fn duplicate_output_symbols_report_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-duplicate-output-symbols.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "main": {
      "graph": {
        "values": [["lhs", 2], ["rhs", 3], "sum"],
        "ops": [["Add", ["lhs", "rhs"], ["sum"]]],
        "input_vals": [],
        "output_vals": ["sum", "sum"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with duplicate outputs");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Duplicate output symbol 'sum'"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn bad_opcode_arity_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-bad-op-arity.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "main": {
      "graph": {
        "values": [["lhs", 2], "sum"],
        "ops": [["Add", ["lhs"], ["sum"]]],
        "input_vals": [],
        "output_vals": ["sum"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with bad opcode arity");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Opcode add"));
    assert!(stderr.contains("expects 2 inputs but received 1"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn duplicate_value_symbols_report_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-duplicate-value-symbols.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "main": {
      "graph": {
        "values": [["lhs", 2], ["lhs", 3], "sum"],
        "ops": [["Add", ["lhs", "lhs"], ["sum"]]],
        "input_vals": [],
        "output_vals": ["sum"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with duplicate values");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Duplicate value symbol 'lhs'"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn missing_value_declarations_report_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-missing-value-declaration.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "main": {
      "graph": {
        "values": [["lhs", 2], ["rhs", 3]],
        "ops": [["Add", ["lhs", "rhs"], ["sum"]]],
        "input_vals": [],
        "output_vals": ["sum"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with missing value declaration");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Output symbol 'sum' is not declared in values"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn missing_op_input_declarations_report_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-missing-input-value.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "main": {
      "graph": {
        "values": [["lhs", 2], "sum"],
        "ops": [["Add", ["lhs", "rhs"], ["sum"]]],
        "input_vals": [],
        "output_vals": ["sum"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with missing op input declaration");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Op input symbol 'rhs' is not declared in values"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn zero_input_call_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-zero-input-call.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "main": {
      "graph": {
        "values": ["result"],
        "ops": [["Call", [], ["result"]]],
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
        .expect("failed to run graphyne await with zero-input call");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Opcode call"));
    assert!(stderr.contains("requires at least 1 inputs but received 0"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn invalid_input_path_exits_non_zero() {
    let output = Command::new(binary_path())
        .args(["await", "-i", "examples/intermediate/does_not_exist.json"])
        .output()
        .expect("failed to run graphyne await with invalid path");

    assert!(!output.status.success(), "expected non-zero exit status for load error");
}
