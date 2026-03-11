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
fn as_dictionary_on_object_drops_type_enforcement() {
    let program_path = std::env::temp_dir().join("graphyne-object-as-dictionary-loses-type.json");
    std::fs::write(
        &program_path,
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
          "as_dict",
          ["age_key", "age"],
          ["bad_age", "thirty seven"],
          "updated"
        ],
        "ops": [
          ["Static", ["Person"], ["Person"]],
          ["Init", ["Person", "name", "age"], ["person"]],
          ["AsDictionary", ["person"], ["as_dict"]],
          ["Set", ["as_dict", "age_key", "bad_age"], ["updated"]]
        ],
        "input_vals": [],
        "output_vals": ["updated"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await for object-as-dictionary semantics");

    assert!(output.status.success(), "expected lossy object-to-dictionary conversion to succeed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"age\":\"thirty seven\""));
    assert!(!stdout.contains("\"type\":"));
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
fn duplicate_collection_symbols_report_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-duplicate-collection-symbols.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "constants": {
    "shared": 1
  },
  "functions": {
    "shared": {
      "graph": {
        "values": [["value", 2]],
        "ops": [],
        "input_vals": [],
        "output_vals": ["value"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with duplicate collection symbols");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Duplicate collection symbol 'shared'"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn call_output_count_mismatch_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-call-output-mismatch.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "double": {
      "graph": {
        "values": ["num", ["two", 2], "result"],
        "ops": [["Mul", ["num", "two"], ["result"]]],
        "input_vals": ["num"],
        "output_vals": ["result"]
      }
    },
    "main": {
      "graph": {
        "values": ["double", ["value", 10], "out1", "out2"],
        "ops": [
          ["Static", ["double"], ["double"]],
          ["Call", ["double", "value"], ["out1", "out2"]]
        ],
        "input_vals": [],
        "output_vals": ["out1", "out2"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with call output mismatch");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Call to 'double'") || stderr.contains("output_types length does not match called_func.output_vals length"));
    assert!(stderr.contains("expects 1 outputs but received 2") || stderr.contains("output_types length does not match called_func.output_vals length"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn call_input_count_mismatch_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-call-input-mismatch.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "double": {
      "graph": {
        "values": ["num", ["two", 2], "result"],
        "ops": [["Mul", ["num", "two"], ["result"]]],
        "input_vals": ["num"],
        "output_vals": ["result"]
      }
    },
    "main": {
      "graph": {
        "values": ["double", "out"],
        "ops": [
          ["Static", ["double"], ["double"]],
          ["Call", ["double"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with call input mismatch");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Call to 'double'"));
    assert!(stderr.contains("expects 2 inputs but received 1"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn missing_static_reference_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-missing-static-reference.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "main": {
      "graph": {
        "values": ["missing_ref", "result"],
        "ops": [["Static", ["missing_ref"], ["result"]]],
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
        .expect("failed to run graphyne await with missing static reference");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Static reference"));
    assert!(stderr.contains("is not declared"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn imported_non_function_call_target_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-imported-non-function-call.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "collections": {
    "lib": {
      "constants": {
        "value": 42
      }
    }
  },
  "imports": {
    "value": ["lib", "value"]
  },
  "functions": {
    "main": {
      "graph": {
        "values": ["value", "result"],
        "ops": [
          ["Static", ["value"], ["value"]],
          ["Call", ["value"], ["result"]]
        ],
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
        .expect("failed to run graphyne await with imported non-function call target");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Call target 'value'"));
    assert!(stderr.contains("is not a function"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn missing_import_target_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-missing-import-target.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "collections": {
    "lib": {
      "constants": {
        "value": 42
      }
    }
  },
  "imports": {
    "missing": ["lib", "does_not_exist"]
  },
  "functions": {
    "main": {
      "graph": {
        "values": ["missing", "result"],
        "ops": [
          ["Static", ["missing"], ["missing"]],
          ["AsString", ["missing"], ["result"]]
        ],
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
        .expect("failed to run graphyne await with missing import target");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Import 'missing'"));
    assert!(stderr.contains("points to missing target"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn imported_call_output_count_mismatch_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-imported-call-output-mismatch.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "collections": {
    "lib": {
      "functions": {
        "double": {
          "graph": {
            "values": ["num", ["two", 2], "result"],
            "ops": [["Mul", ["num", "two"], ["result"]]],
            "input_vals": ["num"],
            "output_vals": ["result"]
          }
        }
      }
    }
  },
  "imports": {
    "double": ["lib", "double"]
  },
  "functions": {
    "main": {
      "graph": {
        "values": ["double", ["value", 10], "out1", "out2"],
        "ops": [
          ["Static", ["double"], ["double"]],
          ["Call", ["double", "value"], ["out1", "out2"]]
        ],
        "input_vals": [],
        "output_vals": ["out1", "out2"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with imported call output mismatch");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Call to 'double'"));
    assert!(stderr.contains("expects 1 outputs but received 2"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn imported_call_input_count_mismatch_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-imported-call-input-mismatch.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "collections": {
    "lib": {
      "functions": {
        "double": {
          "graph": {
            "values": ["num", ["two", 2], "result"],
            "ops": [["Mul", ["num", "two"], ["result"]]],
            "input_vals": ["num"],
            "output_vals": ["result"]
          }
        }
      }
    }
  },
  "imports": {
    "double": ["lib", "double"]
  },
  "functions": {
    "main": {
      "graph": {
        "values": ["double", "out"],
        "ops": [
          ["Static", ["double"], ["double"]],
          ["Call", ["double"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with imported call input mismatch");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Call to 'double'"));
    assert!(stderr.contains("expects 2 inputs but received 1"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn value_assigned_by_multiple_ops_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-multiple-op-writes.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "main": {
      "graph": {
        "values": [["lhs", 2], ["rhs", 3], "sum"],
        "ops": [
          ["Add", ["lhs", "rhs"], ["sum"]],
          ["Mul", ["sum", "rhs"], ["sum"]]
        ],
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
        .expect("failed to run graphyne await with multiple writes to one value");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Value 'sum'"));
    assert!(stderr.contains("assigned 2 times"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn value_assigned_by_input_and_call_output_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-input-call-overwrite.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "id": {
      "graph": {
        "values": ["x"],
        "ops": [],
        "input_vals": ["x"],
        "output_vals": ["x"]
      }
    },
    "main": {
      "graph": {
        "values": ["id", ["x", 5]],
        "ops": [
          ["Static", ["id"], ["id"]],
          ["Call", ["id", "x"], ["x"]]
        ],
        "input_vals": [],
        "output_vals": ["x"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with input overwritten by call output");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Value 'x'"));
    assert!(stderr.contains("assigned 2 times"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn init_target_that_is_known_non_type_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-init-known-non-type.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "main": {
      "graph": {
        "values": [["value", 42], "result"],
        "ops": [
          ["Init", ["value"], ["result"]]
        ],
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
        .expect("failed to run graphyne await with known non-type init target");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Init target 'value'"));
    assert!(stderr.contains("is not a custom type"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn imported_init_target_that_is_not_a_type_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-imported-init-non-type.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "collections": {
    "lib": {
      "constants": {
        "value": 42
      }
    }
  },
  "imports": {
    "value": ["lib", "value"]
  },
  "functions": {
    "main": {
      "graph": {
        "values": ["value", "result"],
        "ops": [
          ["Static", ["value"], ["value"]],
          ["Init", ["value"], ["result"]]
        ],
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
        .expect("failed to run graphyne await with imported non-type init target");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Init target 'value'"));
    assert!(stderr.contains("is not a custom type"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn duplicate_custom_type_fields_report_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-duplicate-type-fields.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "types": {
    "Person": [
      ["name", "str"],
      ["name", "int"]
    ]
  },
  "functions": {
    "main": {
      "graph": {
        "values": ["Person", ["name1", "Ada"], ["name2", 42], "person"],
        "ops": [
          ["Static", ["Person"], ["Person"]],
          ["Init", ["Person", "name1", "name2"], ["person"]]
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
        .expect("failed to run graphyne await with duplicate custom type fields");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Duplicate field 'name'"));
    assert!(stderr.contains("type 'Person'"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn root_relative_custom_type_references_work_in_cli_programs() {
    let program_path = std::env::temp_dir().join("graphyne-root-relative-custom-type-ref.json");
    std::fs::write(
        &program_path,
        r#"{
  "types": {
    "PersonName": {
      "value": "str"
    },
    "Person": {
      "name": "PersonName"
    }
  },
  "functions": {
    "main": {
      "graph": {
        "values": ["PersonName", ["value", "Ada"], "name_obj", "Person", "person"],
        "ops": [
          ["Static", ["PersonName"], ["PersonName"]],
          ["Init", ["PersonName", "value"], ["name_obj"]],
          ["Static", ["Person"], ["Person"]],
          ["Init", ["Person", "name_obj"], ["person"]]
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
        .args(["await", "-i", program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with root-relative custom type reference");

    assert!(output.status.success(), "expected successful execution");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"name\":{\"data\":{\"value\":\"Ada\"}"));
}

#[test]
fn root_symbol_prefixed_custom_type_references_work_in_cli_programs() {
    let program_path = std::env::temp_dir().join("graphyne-root-prefixed-custom-type-ref.json");
    std::fs::write(
        &program_path,
        r#"{
  "types": {
    "PersonName": {
      "value": "str"
    },
    "Person": {
      "name": "root.PersonName"
    }
  },
  "functions": {
    "main": {
      "graph": {
        "values": ["PersonName", ["value", "Ada"], "name_obj", "Person", "person"],
        "ops": [
          ["Static", ["PersonName"], ["PersonName"]],
          ["Init", ["PersonName", "value"], ["name_obj"]],
          ["Static", ["Person"], ["Person"]],
          ["Init", ["Person", "name_obj"], ["person"]]
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
        .args(["await", "-i", program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with root-prefixed custom type reference");

    assert!(output.status.success(), "expected successful execution");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"name\":{\"data\":{\"value\":\"Ada\"}"));
}

#[test]
fn direct_import_cycle_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-direct-import-cycle.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "imports": {
    "a": ["a"]
  },
  "functions": {
    "main": {
      "graph": {
        "values": ["a", "result"],
        "ops": [
          ["Static", ["a"], ["a"]],
          ["AsString", ["a"], ["result"]]
        ],
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
        .expect("failed to run graphyne await with direct import cycle");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Import cycle detected"));
    assert!(stderr.contains("a -> a"));
    assert!(!stderr.contains("stack overflow"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn mutual_import_cycle_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-mutual-import-cycle.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "imports": {
    "a": ["b"],
    "b": ["a"]
  },
  "functions": {
    "main": {
      "graph": {
        "values": ["a", "result"],
        "ops": [
          ["Static", ["a"], ["a"]],
          ["AsString", ["a"], ["result"]]
        ],
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
        .expect("failed to run graphyne await with mutual import cycle");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Import cycle detected"));
    assert!(stderr.contains("a"));
    assert!(stderr.contains("b"));
    assert!(!stderr.contains("stack overflow"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn direct_recursive_call_cycle_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-direct-recursive-call.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "loop": {
      "graph": {
        "values": ["loop", "out"],
        "ops": [
          ["Static", ["loop"], ["loop"]],
          ["Call", ["loop"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    },
    "main": {
      "graph": {
        "values": ["loop", "out"],
        "ops": [
          ["Static", ["loop"], ["loop"]],
          ["Call", ["loop"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with direct recursive call cycle");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Recursive call cycle detected"));
    assert!(stderr.contains("loop -> loop"));
    assert!(!stderr.contains("stack overflow"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn mutual_recursive_call_cycle_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-mutual-recursive-call.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "a": {
      "graph": {
        "values": ["b", "out"],
        "ops": [
          ["Static", ["b"], ["b"]],
          ["Call", ["b"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    },
    "b": {
      "graph": {
        "values": ["a", "out"],
        "ops": [
          ["Static", ["a"], ["a"]],
          ["Call", ["a"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    },
    "main": {
      "graph": {
        "values": ["a", "out"],
        "ops": [
          ["Static", ["a"], ["a"]],
          ["Call", ["a"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with mutual recursive call cycle");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Recursive call cycle detected"));
    assert!(stderr.contains("a") && stderr.contains("b"));
    assert!(!stderr.contains("stack overflow"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn map_target_with_wrong_input_count_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-map-wrong-input-count.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "const_one": {
      "graph": {
        "values": [["value", 1]],
        "ops": [],
        "input_vals": [],
        "output_vals": ["value"]
      }
    },
    "main": {
      "graph": {
        "values": ["const_one", ["items", [1, 2]], "out"],
        "ops": [
          ["Static", ["const_one"], ["const_one"]],
          ["Map", ["const_one", "items"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with wrong map callback input count");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Map target 'const_one'"));
    assert!(stderr.contains("must accept exactly 1 inputs but accepts 0"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn filter_target_with_wrong_input_count_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-filter-wrong-input-count.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "const_true": {
      "graph": {
        "values": [["value", true]],
        "ops": [],
        "input_vals": [],
        "output_vals": ["value"]
      }
    },
    "main": {
      "graph": {
        "values": ["const_true", ["items", [1, 2]], "out"],
        "ops": [
          ["Static", ["const_true"], ["const_true"]],
          ["Filter", ["const_true", "items"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with wrong filter callback input count");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Filter target 'const_true'"));
    assert!(stderr.contains("must accept exactly 1 inputs but accepts 0"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn reduce_target_with_wrong_input_count_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-reduce-wrong-input-count.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "const_one": {
      "graph": {
        "values": [["value", 1]],
        "ops": [],
        "input_vals": [],
        "output_vals": ["value"]
      }
    },
    "main": {
      "graph": {
        "values": ["const_one", ["items", [1, 2]], ["init", 0], "out"],
        "ops": [
          ["Static", ["const_one"], ["const_one"]],
          ["Reduce", ["const_one", "items", "init"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with wrong reduce callback input count");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Reduce target 'const_one'"));
    assert!(stderr.contains("must accept exactly 2 inputs but accepts 0"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn map_target_with_multiple_outputs_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-map-multi-output-callback.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "pair": {
      "graph": {
        "values": ["x", ["one", 1], ["two", 2], "a", "b"],
        "ops": [
          ["Add", ["x", "one"], ["a"]],
          ["Add", ["x", "two"], ["b"]]
        ],
        "input_vals": ["x"],
        "output_vals": ["a", "b"]
      }
    },
    "main": {
      "graph": {
        "values": ["pair", ["items", [1, 2]], "out"],
        "ops": [
          ["Static", ["pair"], ["pair"]],
          ["Map", ["pair", "items"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with multi-output map callback");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Map target 'pair'"));
    assert!(stderr.contains("must produce exactly 1 output but produces 2"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn filter_target_with_multiple_outputs_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-filter-multi-output-callback.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "pair": {
      "graph": {
        "values": ["x", ["one", 1], ["two", 2], "a", "b"],
        "ops": [
          ["Add", ["x", "one"], ["a"]],
          ["Add", ["x", "two"], ["b"]]
        ],
        "input_vals": ["x"],
        "output_vals": ["a", "b"]
      }
    },
    "main": {
      "graph": {
        "values": ["pair", ["items", [1, 2]], "out"],
        "ops": [
          ["Static", ["pair"], ["pair"]],
          ["Filter", ["pair", "items"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with multi-output filter callback");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Filter target 'pair'"));
    assert!(stderr.contains("must produce exactly 1 output but produces 2"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn reduce_target_with_multiple_outputs_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-reduce-multi-output-callback.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "pair": {
      "graph": {
        "values": ["acc", "x", ["one", 1], "a", "b"],
        "ops": [
          ["Add", ["acc", "x"], ["a"]],
          ["Add", ["a", "one"], ["b"]]
        ],
        "input_vals": ["acc", "x"],
        "output_vals": ["a", "b"]
      }
    },
    "main": {
      "graph": {
        "values": ["pair", ["items", [1, 2]], ["init", 0], "out"],
        "ops": [
          ["Static", ["pair"], ["pair"]],
          ["Reduce", ["pair", "items", "init"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with multi-output reduce callback");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Reduce target 'pair'"));
    assert!(stderr.contains("must produce exactly 1 output but produces 2"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn map_target_that_is_known_non_function_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-map-known-non-function.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "main": {
      "graph": {
        "values": [["value", 42], ["items", [1, 2, 3]], "out"],
        "ops": [
          ["Map", ["value", "items"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with non-function map target");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Map target 'value'"));
    assert!(stderr.contains("is not a function"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn filter_target_with_non_bool_constant_output_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-filter-non-bool-callback.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "const_int": {
      "graph": {
        "values": [["value", 1]],
        "ops": [],
        "input_vals": [],
        "output_vals": ["value"]
      }
    },
    "main": {
      "graph": {
        "values": ["const_int", ["items", [1, 2]], "out"],
        "ops": [
          ["Static", ["const_int"], ["const_int"]],
          ["Filter", ["const_int", "items"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with non-bool filter callback");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Filter target 'const_int'"));
    assert!(stderr.contains("must accept exactly 1 inputs but accepts 0") || stderr.contains("must produce a bool output"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn filter_target_that_is_known_non_function_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-filter-known-non-function.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "main": {
      "graph": {
        "values": [["value", 42], ["items", [1, 2, 3]], "out"],
        "ops": [
          ["Filter", ["value", "items"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with non-function filter target");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Filter target 'value'"));
    assert!(stderr.contains("is not a function"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn reduce_target_that_is_known_non_function_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-reduce-known-non-function.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "functions": {
    "main": {
      "graph": {
        "values": [["value", 42], ["items", [1, 2, 3]], ["init", 0], "out"],
        "ops": [
          ["Reduce", ["value", "items", "init"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with non-function reduce target");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Reduce target 'value'"));
    assert!(stderr.contains("is not a function"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn imported_map_target_that_is_not_a_function_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-imported-map-non-function.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "collections": {
    "lib": {
      "constants": {
        "value": 42
      }
    }
  },
  "imports": {
    "value": ["lib", "value"]
  },
  "functions": {
    "main": {
      "graph": {
        "values": ["value", ["items", [1, 2, 3]], "out"],
        "ops": [
          ["Static", ["value"], ["value"]],
          ["Map", ["value", "items"], ["out"]]
        ],
        "input_vals": [],
        "output_vals": ["out"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with imported non-function map target");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Map target 'value'"));
    assert!(stderr.contains("is not a function"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn local_init_arg_count_mismatch_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-local-init-arg-mismatch.json");
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
        "values": ["Person", ["name", "Ada"], "person"],
        "ops": [
          ["Static", ["Person"], ["Person"]],
          ["Init", ["Person", "name"], ["person"]]
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
        .expect("failed to run graphyne await with local init arg mismatch");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Init of 'Person'"));
    assert!(stderr.contains("expects 2 fields but received 1"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn imported_init_arg_count_mismatch_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-imported-init-arg-mismatch.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "collections": {
    "lib": {
      "types": {
        "Person": {
          "name": "str",
          "age": "int"
        }
      }
    }
  },
  "imports": {
    "Person": ["lib", "Person"]
  },
  "functions": {
    "main": {
      "graph": {
        "values": ["Person", ["name", "Ada"], "person"],
        "ops": [
          ["Static", ["Person"], ["Person"]],
          ["Init", ["Person", "name"], ["person"]]
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
        .expect("failed to run graphyne await with imported init arg mismatch");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Init of 'Person'"));
    assert!(stderr.contains("expects 2 fields but received 1"));
    assert!(!stderr.contains("panicked at"));
}

#[test]
fn init_output_count_mismatch_reports_bind_error_cleanly() {
    let invalid_program_path = std::env::temp_dir().join("graphyne-init-output-mismatch.json");
    std::fs::write(
        &invalid_program_path,
        r#"{
  "types": {
    "Person": {
      "name": "str"
    }
  },
  "functions": {
    "main": {
      "graph": {
        "values": ["Person", ["name", "Ada"], "person1", "person2"],
        "ops": [
          ["Static", ["Person"], ["Person"]],
          ["Init", ["Person", "name"], ["person1", "person2"]]
        ],
        "input_vals": [],
        "output_vals": ["person1"]
      }
    }
  }
}"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .args(["await", "-i", invalid_program_path.to_str().unwrap()])
        .output()
        .expect("failed to run graphyne await with init output mismatch");

    assert!(!output.status.success(), "expected non-zero exit status for bind error");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Opcode init"));
    assert!(stderr.contains("expects 1 outputs but received 2"));
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
