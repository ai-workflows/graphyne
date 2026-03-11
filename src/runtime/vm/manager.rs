use std::sync::{Arc, mpsc};
use rayon::ThreadPool;
use crate::runtime::data::live::PointerLive;
use crate::runtime::static_state::state::StaticState;
use crate::runtime::{ExecResult, SymbolPath};
use crate::runtime::vm::orchestrator;
use crate::runtime::vm::outputs::OutputType;

fn get_entry_func(main_symbol_path: &SymbolPath, static_state: &Arc<StaticState>) -> ExecResult<PointerLive> {
    let func_ref = static_state.get_ptr_to_ref(main_symbol_path)
        .map_err(|e| format!("Error loading entry point {:?}: {}", main_symbol_path, e))?;

    func_ref.stored_as_func()
        .map_err(|e| format!("Entry point {:?} is not a function: {}", main_symbol_path, e))?;

    Ok(func_ref)
}

/// Initializes a stream call of a function with the given inputs.
pub fn try_init_stream_call(
    main_symbol_path: SymbolPath,
    inputs: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) -> ExecResult<(usize, mpsc::Receiver<(usize, PointerLive)>)> {
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

    orchestrator::init_anonymous_call(
        func_static_ref,
        &inputs,
        output_types,
        static_state,
        worker_pool,
    );

    Ok((num_outputs, rx))
}

pub fn init_stream_call(
    main_symbol_path: SymbolPath,
    inputs: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) -> (usize, mpsc::Receiver<(usize, PointerLive)>) {
    try_init_stream_call(main_symbol_path, inputs, static_state, worker_pool)
        .unwrap_or_else(|e| panic!("start_call_v2: {}", e))
}

/// Awaits the result of a function call with the given inputs.
pub fn try_init_await_call(
    main_symbol_path: SymbolPath,
    inputs: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) -> ExecResult<Vec<PointerLive>> {
    let (num_outputs, rx) = try_init_stream_call(main_symbol_path, inputs, static_state, worker_pool)?;

    let mut outputs: Vec<Option<PointerLive>> = Vec::with_capacity(num_outputs);
    for _ in 0..num_outputs {
        outputs.push(None);
    }

    for (i, v) in rx.iter().take(num_outputs) {
        outputs[i] = Some(v);
    }

    outputs.into_iter().map(|v|
        v.ok_or_else(|| "init_await_call: function halted execution before all outputs were returned".to_string())
    ).collect()
}

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
