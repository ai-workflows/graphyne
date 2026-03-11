use std::fs;
use std::io::{stderr, stdout, Write};
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use crate::binder::binder;
use crate::binder::intermediate::collection::Collection;
use crate::runtime::data::live::PointerLive;
use crate::runtime::{Symbol, SymbolPath};
use crate::runtime::static_state::state::StaticState;
use crate::runtime::vm::call_context::CallContext;
use crate::runtime::vm::manager::{init_await_call, init_stream_call, try_init_await_call, try_init_stream_call};

type StreamCallResult = (usize, Receiver<(usize, PointerLive)>, Arc<CallContext>);

fn build_worker_pool(workers: Option<usize>) -> Arc<rayon::ThreadPool> {
    let worker_count = get_worker_counts(workers);
    Arc::new(rayon::ThreadPoolBuilder::new().num_threads(worker_count).build().unwrap())
}

pub fn try_await_call(
    main_symbol_path: SymbolPath,
    inputs: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    workers: Option<usize>,
) -> Result<Vec<PointerLive>, String> {
    let worker_pool = build_worker_pool(workers);
    try_init_await_call(main_symbol_path, inputs, static_state, worker_pool)
}

#[allow(dead_code)]
pub fn await_call(
    main_symbol_path: SymbolPath,
    inputs: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    workers: Option<usize>,
) -> Vec<PointerLive> {
    let worker_pool = build_worker_pool(workers);
    init_await_call(main_symbol_path, inputs, static_state, worker_pool)
}

pub fn try_stream_call(
    main_symbol_path: SymbolPath,
    inputs: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    workers: Option<usize>,
) -> Result<StreamCallResult, String> {
    let worker_pool = build_worker_pool(workers);
    try_init_stream_call(main_symbol_path, inputs, static_state, worker_pool)
}

#[allow(dead_code)]
pub fn stream_call(
    main_symbol_path: SymbolPath,
    inputs: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    workers: Option<usize>,
) -> (usize, Receiver<(usize, PointerLive)>) {
    let worker_pool = build_worker_pool(workers);
    init_stream_call(main_symbol_path, inputs, static_state, worker_pool)
}

/// Loads a Graphite JSON Intermediate Language (GJIL) file from the given path and binds to memory.
pub fn load_intermediate(path: &str) -> Result<Collection, String> {
    let contents = fs::read_to_string(path).map_err(|e| format!("Error reading intermediate file: {}", e))?;
    let program: Collection = serde_json::from_str(&contents).map_err(|e| format!("Error parsing intermediate JSON: {}", e))?;

    Ok(program)
}

pub fn bind(program: Collection, program_symbol: Option<Symbol>) -> Result<Arc<StaticState>, String> {
    let mut static_state = StaticState::new();

    match binder::bind_program(program, &mut static_state, program_symbol) {
        Ok(_) => {},
        Err(e) => return Err(format!("Error binding program: {}", e)),
    }

    Ok(Arc::new(static_state))
}

pub fn get_worker_counts(workers: Option<usize>) -> usize {
    match workers {
        Some(0) => 1,
        Some(v) => v,
        None => num_cpus::get(),
    }
}

pub fn log_async(message: String) {
    let out = stdout();
    let _ = writeln!(&mut out.lock(), "{}", message);
}

pub fn log_info(msg: String) {
    let err = stderr();
    let _ = writeln!(&mut err.lock(), "{}", msg);
}

pub fn log_error(msg: String) {
    let err = stderr();
    let _ = writeln!(&mut err.lock(), "\x1B[31m{}\x1B[0m", msg);
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use crate::api::{bind, get_worker_counts, load_intermediate, stream_call, try_await_call, try_stream_call};
    use crate::binder::intermediate::collection::Collection;
    use crate::binder::json::jsonify;
    use crate::runtime::data::live::PointerLive;

    #[test]
    fn test_stream_with_types() {
        let collection: Collection = load_intermediate("examples/intermediate/test_compiled.json").unwrap();

        let static_state = bind(collection, Some("top_level".to_string())).unwrap();

        let (output_count, output_receiver) = stream_call(
            vec!["top_level".to_string(), "main".to_string()],
            vec![],
            static_state.clone(),
            Some(4),
        );

        let mut outputs: HashMap<usize, PointerLive> = HashMap::new();

        for _ in 0..output_count {
            let (idx, val) = output_receiver.recv().unwrap();
            println!("output {}: {}", idx, jsonify(val.as_ref()));
            outputs.insert(idx, val);
        }

        let res = outputs.get(&0).unwrap().stored_as_list().unwrap();
        let res: Vec<i64> = res.iter().map(|v| *v.stored_as_int().unwrap()).collect();
        assert_eq!(res, vec![20, 40, 60]);

        let val = outputs.get(&1).unwrap().stored_as_string().unwrap();
        assert_eq!(val, "World");

        let age = outputs.get(&2).unwrap().stored_as_int().unwrap();
        assert_eq!(*age, 60);
    }

    #[test]
    fn try_await_call_returns_error_for_invalid_entrypoint() {
        let collection: Collection = load_intermediate("examples/intermediate/test_compiled.json").unwrap();
        let static_state = bind(collection, Some("top_level".to_string())).unwrap();

        let err = try_await_call(
            vec!["top_level".to_string(), "does_not_exist".to_string()],
            vec![],
            static_state,
            Some(1),
        ).unwrap_err();

        assert!(err.contains("Error loading entry point"));
    }

    #[test]
    fn zero_workers_falls_back_to_one_thread() {
        assert_eq!(get_worker_counts(Some(0)), 1);
    }

    #[test]
    fn none_workers_uses_cpu_count() {
        assert_eq!(get_worker_counts(None), num_cpus::get());
    }

    fn imported_double_program(import_path: &str) -> String {
        format!(r#"{{
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
        }}"#)
    }

    fn assert_import_program_produces_42(import_path: &str) {
        let collection: Collection = serde_json::from_str(&imported_double_program(import_path)).unwrap();
        let static_state = bind(collection, Some("root".to_string())).unwrap();

        let (count, rx, _context) = try_stream_call(
            vec!["root".to_string(), "main".to_string()],
            vec![],
            static_state,
            Some(1),
        )
        .unwrap();

        assert_eq!(count, 1);
        let (_idx, value) = rx.recv().unwrap();
        assert_eq!(*value.stored_as_int().unwrap(), 42);
    }

    #[test]
    fn try_stream_call_supports_root_relative_import_paths() {
        assert_import_program_produces_42(r#"["lib", "double"]"#);
    }

    #[test]
    fn try_stream_call_supports_import_paths_with_user_visible_root_symbol() {
        assert_import_program_produces_42(r#"["root", "lib", "double"]"#);
    }

    #[test]
    fn bind_rejects_duplicate_input_symbols() {
        let json_collection = r#"{
            "functions": {
                "main": {
                    "graph": {
                        "values": [["lhs", 2], ["rhs", 3], "sum"],
                        "ops": [["Add", ["lhs", "rhs"], ["sum"]]],
                        "input_vals": ["lhs", "lhs"],
                        "output_vals": ["sum"]
                    }
                }
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let err = bind(collection, Some("root".to_string())).err().unwrap();
        assert!(err.contains("Duplicate input symbol 'lhs'"));
    }

    #[test]
    fn bind_rejects_duplicate_output_symbols() {
        let json_collection = r#"{
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
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let err = bind(collection, Some("root".to_string())).err().unwrap();
        assert!(err.contains("Duplicate output symbol 'sum'"));
    }

    #[test]
    fn bind_rejects_bad_opcode_input_arity() {
        let json_collection = r#"{
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
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let err = bind(collection, Some("root".to_string())).err().unwrap();
        assert!(err.contains("Opcode add"));
        assert!(err.contains("expects 2 inputs but received 1"));
    }

    #[test]
    fn bind_rejects_bad_opcode_output_arity() {
        let json_collection = r#"{
            "functions": {
                "main": {
                    "graph": {
                        "values": [["lhs", 2], ["rhs", 3], "sum1", "sum2"],
                        "ops": [["Add", ["lhs", "rhs"], ["sum1", "sum2"]]],
                        "input_vals": [],
                        "output_vals": ["sum1"]
                    }
                }
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let err = bind(collection, Some("root".to_string())).err().unwrap();
        assert!(err.contains("Opcode add"));
        assert!(err.contains("expects 1 outputs but received 2"));
    }

    #[test]
    fn bind_rejects_duplicate_value_symbols() {
        let json_collection = r#"{
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
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let err = bind(collection, Some("root".to_string())).err().unwrap();
        assert!(err.contains("Duplicate value symbol 'lhs'"));
    }

    #[test]
    fn bind_rejects_missing_output_value_declarations() {
        let json_collection = r#"{
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
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let err = bind(collection, Some("root".to_string())).err().unwrap();
        assert!(err.contains("Output symbol 'sum' is not declared in values"));
    }

    #[test]
    fn bind_rejects_missing_input_value_declarations() {
        let json_collection = r#"{
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
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let err = bind(collection, Some("root".to_string())).err().unwrap();
        assert!(err.contains("Op input symbol 'rhs' is not declared in values"));
    }

    #[test]
    fn bind_rejects_zero_input_call() {
        let json_collection = r#"{
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
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let err = bind(collection, Some("root".to_string())).err().unwrap();
        assert!(err.contains("Opcode call"));
        assert!(err.contains("requires at least 1 inputs but received 0"));
    }

    #[test]
    fn bind_rejects_zero_input_init() {
        let json_collection = r#"{
            "functions": {
                "main": {
                    "graph": {
                        "values": ["result"],
                        "ops": [["Init", [], ["result"]]],
                        "input_vals": [],
                        "output_vals": ["result"]
                    }
                }
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let err = bind(collection, Some("root".to_string())).err().unwrap();
        assert!(err.contains("Opcode init"));
        assert!(err.contains("requires at least 1 inputs but received 0"));
    }

    #[test]
    fn bind_rejects_zero_input_static() {
        let json_collection = r#"{
            "functions": {
                "main": {
                    "graph": {
                        "values": ["result"],
                        "ops": [["Static", [], ["result"]]],
                        "input_vals": [],
                        "output_vals": ["result"]
                    }
                }
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let err = bind(collection, Some("root".to_string())).err().unwrap();
        assert!(err.contains("Opcode static"));
        assert!(err.contains("expects 1 inputs but received 0"));
    }
}
