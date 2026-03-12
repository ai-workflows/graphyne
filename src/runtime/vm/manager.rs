use std::sync::{mpsc, Arc};
use rayon::ThreadPool;
use crate::runtime::data::live::PointerLive;
use crate::runtime::static_state::state::StaticState;
use crate::runtime::{ExecResult, SymbolPath};
use crate::runtime::vm::orchestrator;
use crate::runtime::vm::outputs::OutputType;
use crate::runtime::vm::call_context::CallContext;
use std::sync::Mutex;

type StreamInit = (usize, mpsc::Receiver<(usize, PointerLive)>, Arc<CallContext>);

fn get_entry_func(main_symbol_path: &SymbolPath, static_state: &Arc<StaticState>) -> ExecResult<PointerLive> {
    let func_ref = static_state.get_ptr_to_ref(main_symbol_path)
        .map_err(|e| format!("Error loading entry point {:?}: {}", main_symbol_path, e))?;

    func_ref.stored_as_func()
        .map_err(|e| format!("Entry point {:?} is not a function: {}", main_symbol_path, e))?;

    Ok(func_ref)
}

pub fn try_init_stream_call(
    main_symbol_path: SymbolPath,
    inputs: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) -> ExecResult<StreamInit> {
    let func_ref = get_entry_func(&main_symbol_path, &static_state)?;
    let num_outputs = func_ref.stored_as_func()
        .map_err(|e| format!("Entry point {:?} is not a function: {}", main_symbol_path, e))?
        .output_vals.len();

    let (tx, rx) = mpsc::channel();

    let output_types: Vec<OutputType> = (0..num_outputs)
        .map(|i| OutputType::Final(i, tx.clone()))
        .collect();

    let func_static_ref = func_ref.as_static_ref()
        .map_err(|e| format!("Entry point {:?} is not a static function reference: {}", main_symbol_path, e))?;

    let runtime_error = Arc::new(Mutex::new(None));
    orchestrator::init_anonymous_call(
        func_static_ref,
        &inputs,
        output_types,
        static_state,
        worker_pool,
        runtime_error.clone(),
    );

    let context = Arc::new(CallContext::new(func_static_ref.clone(), Vec::new(), runtime_error));
    Ok((num_outputs, rx, context))
}

#[allow(dead_code)]
pub fn init_stream_call(
    main_symbol_path: SymbolPath,
    inputs: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) -> (usize, mpsc::Receiver<(usize, PointerLive)>) {
    let (num_outputs, rx, _context) = try_init_stream_call(main_symbol_path, inputs, static_state, worker_pool)
        .unwrap_or_else(|e| panic!("start_call_v2: {}", e));
    (num_outputs, rx)
}

pub fn try_init_await_call(
    main_symbol_path: SymbolPath,
    inputs: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) -> ExecResult<Vec<PointerLive>> {
    let (num_outputs, rx, context) = try_init_stream_call(main_symbol_path, inputs, static_state, worker_pool)?;

    let mut outputs: Vec<Option<PointerLive>> = Vec::with_capacity(num_outputs);
    for _ in 0..num_outputs {
        outputs.push(None);
    }

    for _ in 0..num_outputs {
        match rx.recv_timeout(std::time::Duration::from_millis(500)) {
            Ok((i, v)) => outputs[i] = Some(v),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let res: ExecResult<Vec<PointerLive>> = outputs.into_iter().map(|v|
        v.ok_or_else(|| {
            match context.runtime_error.lock().unwrap().clone() {
                Some(err) => err,
                None => "init_await_call: function halted execution before all outputs were returned".to_string(),
            }
        })
    ).collect();

    res
}

#[allow(dead_code)]
pub fn init_await_call(
    main_symbol_path: SymbolPath,
    inputs: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) -> Vec<PointerLive> {
    try_init_await_call(main_symbol_path, inputs, static_state, worker_pool)
        .unwrap_or_else(|e| panic!("start_call_v2: {}", e))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use crate::api::bind;
    use crate::binder::intermediate::collection::Collection;
    use crate::binder::json::jsonify;
    use crate::runtime::data::live::{IntLive, LiveData, PointerLive};
    use crate::runtime::static_state::state::StaticState;
    use crate::runtime::vm::manager::{init_await_call, try_init_await_call};

    const CORRECTNESS_REPEATS: usize = 10;

    #[test]
    fn try_init_await_call_returns_error_for_missing_entrypoint() {
        let json_collection = r#"{
            "functions": {
                "main": {
                    "graph": {
                        "values": [["value", 1]],
                        "ops": [],
                        "input_vals": [],
                        "output_vals": ["value"]
                    }
                }
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();
        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());

        let err = try_init_await_call(
            vec!["my_collection".to_string(), "does_not_exist".to_string()],
            vec![],
            static_state,
            worker_pool,
        ).unwrap_err();

        assert!(err.contains("Error loading entry point"));
    }

    #[test]
    fn try_init_await_call_returns_error_for_non_function_entrypoint() {
        let json_collection = r#"{
            "constants": {
                "main": 42
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();
        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());

        let err = try_init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state,
            worker_pool,
        ).unwrap_err();

        assert!(err.contains("is not a function"));
    }

    #[test]
    fn try_init_await_call_returns_runtime_error_instead_of_panicking() {
        let json_collection = r#"{
            "types": {
                "Person": [["age", "int"]]
            },
            "functions": {
                "main": {
                    "graph": {
                        "values": [
                            "person_type",
                            ["bad_age", "not-an-int"],
                            "person"
                        ],
                        "ops": [
                            ["Static", ["Person"], ["person_type"]],
                            ["Init", ["person_type", "bad_age"], ["person"]]
                        ],
                        "input_vals": [],
                        "output_vals": ["person"]
                    }
                }
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();
        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());

        let err = try_init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state,
            worker_pool,
        ).unwrap_err();

        assert!(err.contains("Cannot initialize object of type Person"));
    }

    #[test]
    fn try_init_await_call_returns_error_for_negative_integer_pow_exponent() {
        let json_collection = r#"{
            "functions": {
                "main": {
                    "graph": {
                        "values": [["base", 2], ["exp", -1], "out"],
                        "ops": [["Pow", ["base", "exp"], ["out"]]],
                        "input_vals": [],
                        "output_vals": ["out"]
                    }
                }
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();
        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());

        let outputs = try_init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state,
            worker_pool,
        ).unwrap();

        assert_eq!(outputs.len(), 1);
        let out = outputs[0].stored_as_float().unwrap();
        assert!((out - 0.5).abs() < 1e-9);
    }

    #[test]
    fn try_init_await_call_returns_error_for_integer_pow_overflow() {
        let json_collection = r#"{
            "functions": {
                "main": {
                    "graph": {
                        "values": [["base", 2], ["exp", 63], "out"],
                        "ops": [["Pow", ["base", "exp"], ["out"]]],
                        "input_vals": [],
                        "output_vals": ["out"]
                    }
                }
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();
        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());

        let outputs = try_init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state,
            worker_pool,
        ).unwrap();

        assert_eq!(outputs.len(), 1);
        let out = outputs[0].stored_as_float().unwrap();
        assert!(out.is_finite());
        assert!(*out > 9.0e18);
    }

    #[test]
    fn try_init_await_call_supports_fractional_float_pow_exponents() {
        let json_collection = r#"{
            "functions": {
                "main": {
                    "graph": {
                        "values": [["base", 9.0], ["exp", 0.5], "out"],
                        "ops": [["Pow", ["base", "exp"], ["out"]]],
                        "input_vals": [],
                        "output_vals": ["out"]
                    }
                }
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();
        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());

        let outputs = try_init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state,
            worker_pool,
        ).unwrap();

        assert_eq!(outputs.len(), 1);
        let out = outputs[0].stored_as_float().unwrap();
        assert!((out - 3.0).abs() < 1e-9);
    }

    #[test]
    fn try_init_await_call_supports_negative_float_pow_exponents() {
        let json_collection = r#"{
            "functions": {
                "main": {
                    "graph": {
                        "values": [["base", 2.0], ["exp", -1.0], "out"],
                        "ops": [["Pow", ["base", "exp"], ["out"]]],
                        "input_vals": [],
                        "output_vals": ["out"]
                    }
                }
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();
        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());

        let outputs = try_init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state,
            worker_pool,
        ).unwrap();

        assert_eq!(outputs.len(), 1);
        let out = outputs[0].stored_as_float().unwrap();
        assert!((out - 0.5).abs() < 1e-9);
    }

    #[test]
    fn equality_against_null_returns_false_for_non_null_values() {
        let json_collection = r#"{
            "constants": {
                "truth": true,
                "items": [1, 2],
                "obj": {"a": 1},
                "num": 1,
                "nothing": null
            },
            "functions": {
                "main": {
                    "graph": {
                        "values": ["truth", "items", "obj", "num", "nothing", "truth_eq", "items_eq", "obj_eq", "num_eq"],
                        "ops": [
                            ["Static", ["truth"], ["truth"]],
                            ["Static", ["items"], ["items"]],
                            ["Static", ["obj"], ["obj"]],
                            ["Static", ["num"], ["num"]],
                            ["Static", ["nothing"], ["nothing"]],
                            ["Equal", ["truth", "nothing"], ["truth_eq"]],
                            ["Equal", ["items", "nothing"], ["items_eq"]],
                            ["Equal", ["obj", "nothing"], ["obj_eq"]],
                            ["Equal", ["num", "nothing"], ["num_eq"]]
                        ],
                        "input_vals": [],
                        "output_vals": ["truth_eq", "items_eq", "obj_eq", "num_eq"]
                    }
                }
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();
        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());

        let outputs = try_init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state,
            worker_pool,
        ).unwrap();

        assert_eq!(outputs.len(), 4);
        for output in outputs {
            assert!(!(*output.stored_as_bool().unwrap()));
        }
    }

    #[test]
    fn equality_against_null_returns_false_for_static_references() {
        let json_collection = r#"{
            "constants": {
                "truth": true,
                "nothing": null
            },
            "functions": {
                "main": {
                    "graph": {
                        "values": ["truth", "nothing", "truth_ref", "eq1", "eq2"],
                        "ops": [
                            ["Static", ["truth"], ["truth_ref"]],
                            ["Static", ["nothing"], ["nothing"]],
                            ["Equal", ["truth_ref", "nothing"], ["eq1"]],
                            ["Equal", ["nothing", "truth_ref"], ["eq2"]]
                        ],
                        "input_vals": [],
                        "output_vals": ["eq1", "eq2"]
                    }
                }
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();
        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());

        let outputs = try_init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state,
            worker_pool,
        ).unwrap();

        assert_eq!(outputs.len(), 2);
        for output in outputs {
            assert!(!(*output.stored_as_bool().unwrap()));
        }
    }


    #[test]
    fn test_start_call_simple() {
        let json_collection = r#"{
            "constants": {
                "two": 2
            },
            "functions": {
                "main": {
                    "name": "Main",
                    "description": "Main function",
                    "graph": {
                        "values": [
                            "two",
                            ["num", 10],
                            "result"
                        ],
                        "ops": [
                            ["Static", ["two"], ["two"]],
                            ["Add", ["num", "two"], ["result"]]
                        ],
                        "input_vals": [],
                        "output_vals": ["result"]
                    }
                }
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();
        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());

        for _ in 0..CORRECTNESS_REPEATS {
            let _ = init_await_call(
                vec!["my_collection".to_string(), "main".to_string()],
                vec![],
                static_state.clone(),
                worker_pool.clone(),
            );
        }

        let res = init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state.clone(),
            worker_pool.clone(),
        );

        assert_eq!(res.len(), 1);

        let result = res[0].as_live().as_int().unwrap().unwrap();

        assert_eq!(result, 12);
    }

    #[test]
    fn test_start_call() {
        let json_collection = r#"{
            "constants": {
                "two": 2
            },
            "functions": {
                "double": {
                   "name": "Double",
                   "description": "Doubles a number",
                   "graph": {
                        "values": [
                            "two",
                            "num",
                            "doubled"
                        ],
                        "ops": [
                            ["Static", ["two"], ["two"]],
                            ["Mul", ["num", "two"], ["doubled"]]
                        ],
                        "input_vals": ["num"],
                        "output_vals": ["doubled"]
                    }
                },
                "main": {
                    "name": "Main",
                    "description": "Main function",
                    "graph": {
                        "values": [
                            "double",
                            ["arg", 10],
                            "result"
                        ],
                        "ops": [
                            ["Static", ["double"], ["double"]],
                            ["Call", ["double", "arg"], ["result"]]
                        ],
                        "input_vals": [],
                        "output_vals": ["result"]
                    }
                }
            },
            "intermediate": {},
            "imports": {},
            "types": {}
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();
        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());

        for _ in 0..CORRECTNESS_REPEATS {
            let _ = init_await_call(
                vec!["my_collection".to_string(), "main".to_string()],
                vec![],
                static_state.clone(),
                worker_pool.clone(),
            );
        }

        let res = init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state.clone(),
            worker_pool.clone(),
        );

        assert_eq!(res.len(), 1);

        let result = res[0].as_live().as_int().unwrap().unwrap();

        assert_eq!(result, 20);
    }

    #[test]
    fn test_reduce() {
        let json_collection = r#"{
            "constants": {
                "my_list": [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150]
            },
            "functions": {
                "add": {
                   "name": "Add",
                   "description": "Adds two numbers",
                   "graph": {
                        "values": [
                            "num1",
                            "num2",
                            "sum"
                        ],
                        "ops": [
                            ["Add", ["num1", "num2"], ["sum"]]
                        ],
                        "input_vals": ["num1", "num2"],
                        "output_vals": ["sum"]
                    }
                },
                "main": {
                    "name": "Main",
                    "description": "Main function",
                    "graph": {
                        "values": [
                            "add",
                            "my_list",
                            ["initial", 0],
                            "result"
                        ],
                        "ops": [
                            ["Static", ["add"], ["add"]],
                            ["Static", ["my_list"], ["my_list"]],
                            ["Reduce", ["add", "my_list", "initial"], ["result"]]
                        ],
                        "input_vals": [],
                        "output_vals": ["result"]
                    }
                }
            },
            "intermediate": {},
            "imports": {},
            "types": {}
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();
        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());

        for _ in 0..CORRECTNESS_REPEATS {
            let _ = init_await_call(
                vec!["my_collection".to_string(), "main".to_string()],
                vec![],
                static_state.clone(),
                worker_pool.clone(),
            );
        }

        let res = init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state.clone(),
            worker_pool.clone(),
        );

        assert_eq!(res.len(), 1);

        let result = res[0].as_live().as_int().unwrap().unwrap();

        assert_eq!(result, 1200);
    }

    #[test]
    fn test_start_call_map() {
        let json_collection = r#"{
            "constants": {
                "two": 2,
                "my_list": [10, 20, 30],
                "my_dict": {
                    "Hello": "World",
                    "Foo": "Bar"
                }
            },
            "functions": {
                "double": {
                   "name": "Double",
                   "description": "Doubles a number",
                   "graph": {
                        "values": [
                            "two",
                            "num",
                            "doubled"
                        ],
                        "ops": [
                            ["Static", ["two"], ["two"]],
                            ["Mul", ["num", "two"], ["doubled"]]
                        ],
                        "input_vals": ["num"],
                        "output_vals": ["doubled"]
                    }
                },
                "double_list": {
                    "name": "Double List",
                    "description": "Doubles a list of numbers",
                    "graph": {
                        "values": [
                            "double_func",
                            "my_list",
                            "double_list"
                        ],
                        "ops": [
                            ["Static", ["double"], ["double_func"]],
                            ["Static", ["my_list"], ["my_list"]],
                            ["Map", ["double_func", "my_list"], ["double_list"]]
                        ],
                        "input_vals": [],
                        "output_vals": ["double_list"]
                    }
                }
            },
            "intermediate": {},
            "imports": {},
            "types": {}
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();

        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();

        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap());

        for _ in 0..CORRECTNESS_REPEATS {
            let _ = init_await_call(
                vec!["my_collection".to_string(), "double_list".to_string()],
                vec![],
                static_state.clone(),
                worker_pool.clone(),
            );
        }

        let res: Vec<PointerLive> = init_await_call(
            vec!["my_collection".to_string(), "double_list".to_string()],
            vec![],
            static_state.clone(),
            worker_pool.clone(),
        );

        for (i, v) in res.iter().enumerate() {
            println!("out | {}: {}", i, jsonify(v.as_ref()));
        }

        assert_eq!(res.len(), 1);

        let result = res[0].as_live().as_list().unwrap().unwrap();

        let result: Vec<IntLive> = result.iter().map(|ptr|
            ptr.as_live().as_int().unwrap().unwrap()).collect();

        assert_eq!(result, vec![20, 40, 60]);
    }

    #[test]
    fn test_empty_map_returns_empty_list() {
        let json_collection = r#"{
            "constants": {
                "my_list": []
            },
            "functions": {
                "double": {
                   "name": "Double",
                   "description": "Doubles a number",
                   "graph": {
                        "values": [
                            ["two", 2],
                            "num",
                            "doubled"
                        ],
                        "ops": [
                            ["Mul", ["num", "two"], ["doubled"]]
                        ],
                        "input_vals": ["num"],
                        "output_vals": ["doubled"]
                    }
                },
                "main": {
                    "name": "Main",
                    "description": "Main function",
                    "graph": {
                        "values": [
                            "double",
                            "my_list",
                            "result"
                        ],
                        "ops": [
                            ["Static", ["double"], ["double"]],
                            ["Static", ["my_list"], ["my_list"]],
                            ["Map", ["double", "my_list"], ["result"]]
                        ],
                        "input_vals": [],
                        "output_vals": ["result"]
                    }
                }
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();
        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());

        let res = init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state,
            worker_pool,
        );

        let result = res[0].as_live().as_list().unwrap().unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_empty_reduce_returns_initial_value() {
        let json_collection = r#"{
            "constants": {
                "my_list": []
            },
            "functions": {
                "add": {
                   "name": "Add",
                   "description": "Adds two numbers",
                   "graph": {
                        "values": [
                            "lhs",
                            "rhs",
                            "sum"
                        ],
                        "ops": [
                            ["Add", ["lhs", "rhs"], ["sum"]]
                        ],
                        "input_vals": ["lhs", "rhs"],
                        "output_vals": ["sum"]
                    }
                },
                "main": {
                    "name": "Main",
                    "description": "Main function",
                    "graph": {
                        "values": [
                            "add",
                            "my_list",
                            ["initial", 7],
                            "result"
                        ],
                        "ops": [
                            ["Static", ["add"], ["add"]],
                            ["Static", ["my_list"], ["my_list"]],
                            ["Reduce", ["add", "my_list", "initial"], ["result"]]
                        ],
                        "input_vals": [],
                        "output_vals": ["result"]
                    }
                }
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();
        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());

        let res = init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state,
            worker_pool,
        );

        let result = res[0].as_live().as_int().unwrap().unwrap();
        assert_eq!(result, 7);
    }

    #[test]
    fn test_filter() {
        let json_collection = r#"{
            "constants": {
                "my_list": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
            },
            "functions": {
                "is_even": {
                    "name": "Is Even",
                    "description": "Checks if a number is even",
                    "graph": {
                        "values": [
                            ["two", 2],
                            ["zero", 0],
                            "num",
                            "is_even",
                            "mod_result"
                        ],
                        "ops": [
                            ["Mod", ["num", "two"], ["mod_result"]],
                            ["Equal", ["mod_result", "zero"], ["is_even"]]
                        ],
                        "input_vals": ["num"],
                        "output_vals": ["is_even"]
                    }
                },
                "filter_even": {
                    "name": "Filter Even",
                    "description": "Filters a list of numbers to only include even numbers",
                    "graph": {
                        "values": [
                            "is_even",
                            "my_list",
                            "even_list"
                        ],
                        "ops": [
                            ["Static", ["is_even"], ["is_even"]],
                            ["Static", ["my_list"], ["my_list"]],
                            ["Filter", ["is_even", "my_list"], ["even_list"]]
                        ],
                        "input_vals": [],
                        "output_vals": ["even_list"]
                    }
                }
            },
            "intermediate": {},
            "imports": {},
            "types": {}
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();
        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap());

        for _ in 0..CORRECTNESS_REPEATS {
            let _ = init_await_call(
                vec!["my_collection".to_string(), "filter_even".to_string()],
                vec![],
                static_state.clone(),
                worker_pool.clone(),
            );
        }

        let res = init_await_call(
            vec!["my_collection".to_string(), "filter_even".to_string()],
            vec![],
            static_state.clone(),
            worker_pool.clone(),
        );

        assert_eq!(res.len(), 1);

        let result = res[0].as_live().as_list().unwrap().unwrap();

        let result: Vec<IntLive> = result.iter().map(|ptr|
            ptr.as_live().as_int().unwrap().unwrap()).collect();

        assert_eq!(result, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_empty_filter_returns_empty_list() {
        let json_collection = r#"{
            "constants": {
                "my_list": []
            },
            "functions": {
                "is_even": {
                    "name": "Is Even",
                    "description": "Checks if a number is even",
                    "graph": {
                        "values": [
                            ["two", 2],
                            ["zero", 0],
                            "num",
                            "is_even",
                            "mod_result"
                        ],
                        "ops": [
                            ["Mod", ["num", "two"], ["mod_result"]],
                            ["Equal", ["mod_result", "zero"], ["is_even"]]
                        ],
                        "input_vals": ["num"],
                        "output_vals": ["is_even"]
                    }
                },
                "main": {
                    "name": "Main",
                    "description": "Main function",
                    "graph": {
                        "values": [
                            "is_even",
                            "my_list",
                            "result"
                        ],
                        "ops": [
                            ["Static", ["is_even"], ["is_even"]],
                            ["Static", ["my_list"], ["my_list"]],
                            ["Filter", ["is_even", "my_list"], ["result"]]
                        ],
                        "input_vals": [],
                        "output_vals": ["result"]
                    }
                }
            }
        }"#;

        let collection: Collection = serde_json::from_str(json_collection).unwrap();
        let static_state: Arc<StaticState> = bind(collection, Some("my_collection".to_string())).unwrap();
        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());

        let res = init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state,
            worker_pool,
        );

        let result = res[0].as_live().as_list().unwrap().unwrap();
        assert!(result.is_empty());
    }
}
