use std::sync::{Arc, mpsc};
use rayon::ThreadPool;
use crate::runtime::data::functions::func::FuncLive;
use crate::runtime::data::live::{PointerLive};
use crate::runtime::static_state::state::StaticState;
use crate::runtime::SymbolPath;
use crate::runtime::vm::call_context::{CallContext};
use crate::runtime::vm::orchestrator;
use crate::runtime::vm::outputs::OutputType;

/// Initializes a stream call of a function with the given inputs.
pub fn init_stream_call(
    main_symbol_path: SymbolPath,
    inputs: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) -> (usize, mpsc::Receiver<(usize, PointerLive)>) {
    let func_ref: PointerLive = match static_state.get_ptr_to_ref(&main_symbol_path) {
        Ok(v) => v,
        Err(e) => panic!("start_call_v2: {}", e)
    };

    let func: &FuncLive = match func_ref.stored_as_funcv2() {
        Ok(v) => v,
        Err(e) => panic!("start_call_v2: {}", e)
    };

    let num_outputs: usize = func.output_vals.len();

    let (tx, rx) = mpsc::channel();

    let output_types: Vec<OutputType> = func.output_vals.iter().enumerate().map(|(i, _)| {
        OutputType::Final(i, tx.clone())
    }).collect();

    let context: Arc<CallContext> = Arc::new(CallContext::new(
        func_ref.as_static_ref().unwrap().clone(),
        output_types));

    orchestrator::init_call(context.clone(), &inputs, static_state, worker_pool);

    (num_outputs, rx)
}

/// Awaits the result of a function call with the given inputs.
pub fn init_await_call(
    main_symbol_path: SymbolPath,
    inputs: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>
) -> Vec<PointerLive> {
    let (num_outputs, rx) = init_stream_call(main_symbol_path, inputs, static_state, worker_pool);

    // initialize vector of length num_outputs
    let mut outputs: Vec<Option<PointerLive>> = Vec::with_capacity(num_outputs);
    for _ in 0..num_outputs {
        outputs.push(None);
    }

    for (i, v) in rx.iter().take(num_outputs) {
        outputs[i] = Some(v);
    }

    outputs.into_iter().map(|v| v.unwrap()).collect()
}


#[cfg(test)]
mod tests {
    use std::sync::{Arc};
    use crate::api::bind;
    use crate::binder::intermediate::collection::Collection;
    use crate::binder::json::jsonify;
    use crate::runtime::data::live::{LiveData, IntLive, PointerLive};
    use crate::runtime::static_state::state::StaticState;
    use crate::runtime::vm::manager::init_await_call;

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

        let start_time = std::time::Instant::now();
        for _ in 0..10000 {
            let _ = init_await_call(
                vec!["my_collection".to_string(), "main".to_string()],
                vec![],
                static_state.clone(),
                worker_pool.clone()
            );
        }
        let elapsed = start_time.elapsed();
        println!("Average time: {:?}", elapsed / 10000);

        let res = init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state.clone(),
            worker_pool.clone()
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

        let start_time = std::time::Instant::now();
        for _ in 0..10000 {
            let _ = init_await_call(
                vec!["my_collection".to_string(), "main".to_string()],
                vec![],
                static_state.clone(),
                worker_pool.clone()
            );
        }

        let elapsed = start_time.elapsed();
        println!("Average time: {:?}", elapsed / 10000);

        let res = init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state.clone(),
            worker_pool.clone()
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

        let start_time = std::time::Instant::now();

        for _ in 0..10000 {
            let _ = init_await_call(
                vec!["my_collection".to_string(), "main".to_string()],
                vec![],
                static_state.clone(),
                worker_pool.clone()
            );
        }

        let elapsed = start_time.elapsed();
        println!("Average time: {:?}", elapsed / 10000);

        let res = init_await_call(
            vec!["my_collection".to_string(), "main".to_string()],
            vec![],
            static_state.clone(),
            worker_pool.clone()
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

        let start_time = std::time::Instant::now();

        for _ in 0..10000 {
            let _ = init_await_call(
                vec!["my_collection".to_string(), "double_list".to_string()],
                vec![],
                static_state.clone(),
                worker_pool.clone()
            );
        }

        let elapsed = start_time.elapsed();
        println!("Average time: {:?}", elapsed / 10000);

        let res: Vec<PointerLive> = init_await_call(
            vec!["my_collection".to_string(), "double_list".to_string()],
            vec![],
            static_state.clone(),
            worker_pool.clone()
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

        let start_time = std::time::Instant::now();
        for _ in 0..10000 {
            let _ = init_await_call(
                vec!["my_collection".to_string(), "filter_even".to_string()],
                vec![],
                static_state.clone(),
                worker_pool.clone()
            );
        }

        let elapsed = start_time.elapsed();
        println!("Average time: {:?}", elapsed / 10000);

        let res = init_await_call(
            vec!["my_collection".to_string(), "filter_even".to_string()],
            vec![],
            static_state.clone(),
            worker_pool.clone()
        );

        assert_eq!(res.len(), 1);

        let result = res[0].as_live().as_list().unwrap().unwrap();

        let result: Vec<IntLive> = result.iter().map(|ptr|
            ptr.as_live().as_int().unwrap().unwrap()).collect();

        assert_eq!(result, vec![2, 4, 6, 8, 10]);
    }
}
