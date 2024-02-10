use std::collections::HashSet;
use std::sync::{Arc, mpsc};
use std::thread;
use std::io;
use std::io::Write;
use rayon::ThreadPool;
use crate::core::data::functions::val::FuncValId;
use crate::core::data::live::FuncValLive;
use crate::core::ExecResult;
use crate::core::vm::mmu::mmu::{MMU};
use crate::core::vm::v2::{executor, orchestrator};
use crate::core::vm::v2::orchestrator::handle_called_fn_constants;
use crate::core::vm::v2::shared::{CallContextId, get_func_from_ptr, get_func_vals_from_ptrs, NewOpMessage, NewValMessage, SharedCallState};
use crate::core::vm::value_ref::ValueReference;

pub fn start_call<'a>(
    mmu: Arc<MMU>,
    max_workers: usize,
    func: ValueReference,
    args: Vec<ValueReference>,
    output_callback: Arc<impl Fn(&NewValMessage) + Send + Sync + 'static>,
    result_callback: Arc<impl Fn(ExecResult<()>) + Send + Sync + 'static>,
) {
    // generate a random call context id
    let main_call_id = uuid::Uuid::new_v4().to_string();

    // get the function's outputs
    let func_live = get_func_from_ptr(mmu.clone(), &func.pointer).unwrap();
    let output_fn_vals = get_func_vals_from_ptrs(
        mmu.clone(),
        &func_live.output_vals,
    ).unwrap();

    let final_outputs: HashSet<(CallContextId, FuncValId)> = output_fn_vals.iter()
        .map(|val| (main_call_id.clone(), val.guid.clone()))
        .collect();

    // initialize the message channels
    let (new_op_sender, new_op_receiver) = mpsc::channel::<NewOpMessage>();
    let (new_val_sender, new_val_receiver) = mpsc::channel::<NewValMessage>();

    let shared_state: Arc<SharedCallState> = SharedCallState::new(
        mmu.clone(),
        new_op_sender,
        new_val_sender,
        final_outputs,
    );

    // start the orchestrator and executor threads
    start_orchestrator(max_workers, shared_state.clone(), new_val_receiver, output_callback, result_callback.clone());
    start_executor(max_workers, shared_state.clone(), new_op_receiver, result_callback.clone());

    // get the function's inputs
    let input_fn_vals = get_func_vals_from_ptrs(
        mmu.clone(),
        &func_live.input_vals,
    ).unwrap();

    // match the function's inputs with the provided args
    let input_fn_vals: Vec<(ValueReference, FuncValLive)> = args.iter()
        .zip(input_fn_vals)
        .map(|(arg, val)| (arg.clone(), val.clone()))
        .collect();

    // send the inputs as new values
    for (val_ref, func_val) in input_fn_vals {
        shared_state.send_new_val(main_call_id.clone(), func_val, val_ref);
    }

    // send the function's constants as new values
    handle_called_fn_constants(shared_state.clone(), &main_call_id, &func_live)
        .unwrap();
}

pub fn start_orchestrator(
    max_workers: usize,
    shared_state: Arc<SharedCallState>,
    new_val_receiver: mpsc::Receiver<NewValMessage>,
    output_callback: Arc<impl Fn(&NewValMessage) + Send + Sync + 'static>,
    result_callback: Arc<impl Fn(ExecResult<()>) + Send + Sync + 'static>,
) {
    let or_pool: ThreadPool = rayon::ThreadPoolBuilder::new().num_threads(max_workers).build().unwrap();
    // Orchestrator Dispatcher thread
    thread::spawn(move || {
        for message in new_val_receiver.iter() {
            let ss = shared_state.clone();
            let result_callback = result_callback.clone();

            // call the output callback if the message is a final output
            if ss.check_for_final_output(&message.call_context_id, &message.func_val) {
                output_callback(&message);
            }

            // if there are no remaining final outputs, call the result callback and halt execution
            if !ss.has_remaining_final_outputs() {
                result_callback(Ok(()));
                ss.halt_execution(&message.call_context_id, "Execution completed successfully".to_string());
            }

            or_pool.spawn(move || {
                match orchestrator::handle_new_value_v2(
                    ss.clone(),
                    &message.call_context_id,
                    &message.func_val,
                    message.value,
                ) {
                    Ok(_) => {},
                    Err(e) => {
                        // if an error occurred, handle it
                        let error_msg = format!("Orchestrator encountered an error: {}", e);
                        ss.halt_execution(&message.call_context_id, error_msg.clone());
                        result_callback(Err(error_msg));
                    }
                }
            });
        }
    });
}

pub fn start_executor(
    max_workers: usize,
    shared_state: Arc<SharedCallState>,
    new_op_receiver: mpsc::Receiver<NewOpMessage>,
    result_callback: Arc<impl Fn(ExecResult<()>) + Send + Sync + 'static>,
) {
    let ex_pool: ThreadPool = rayon::ThreadPoolBuilder::new().num_threads(max_workers).build().unwrap();
    // Executor Dispatcher thread
    thread::spawn(move || {
        for message in new_op_receiver.iter() {
            let ss = shared_state.clone();
            let result_callback = result_callback.clone();

            ex_pool.spawn(move || {
                match executor::try_execute_fn_op(ss.clone(), &message.op, &message.call_context_id) {
                    Ok(results) => {
                        // if successful, send the results to the state manager
                        for (val_ref, func_val) in results {
                            ss.send_new_val(message.call_context_id.clone(), func_val, val_ref);
                        }
                    },
                    Err(e) => {
                        // if an error occurred, handle it
                        let error_msg = format!("Executor encountered an error: {}", e);
                        ss.halt_execution(&message.call_context_id, error_msg.clone());
                        result_callback(Err(error_msg));
                    }
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Condvar, Mutex};
    use crate::api::collections::collection::Collection;
    use crate::api::GraphiteApi;
    use crate::core::data::live::{LiveData, IntLive};
    use crate::core::vm::mmu::mmu::MMU;
    use crate::core::vm::v2::manager::start_call;
    use crate::core::vm::v2::shared::{log_async, NewValMessage};
    use crate::core::vm::value_ref::ValueReference;

    #[test]
    fn test_start_call_simple() {
        let mmu: Arc<MMU> = Arc::new(MMU::new());

        {
            let mut api = GraphiteApi { mmu, symbol_table: HashMap::new() };

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
                                ["_two", "two"],
                                "two",
                                ["num", 10],
                                "result"
                            ],
                            "ops": [
                                ["Get", ["outer", "_two"], "two"],
                                ["Add", ["num", "two"], "result"]
                            ],
                            "input_vals": [],
                            "output_vals": ["result"]
                        }
                    }
                }
            }"#;

            let collection: Collection = serde_json::from_str(json_collection).unwrap();

            api.store_collection(collection, "my_collection".to_string()).unwrap();

            let main_ref = api.get_path(vec!["my_collection".into(), "main".into()]).unwrap();

            let outputs: Arc<Mutex<Vec<ValueReference>>> = Arc::new(Mutex::new(vec![]));
            let o2 = outputs.clone();

            let output_callback = Arc::new(move |message: &NewValMessage| {
                let mut outputs_guard = outputs.lock().unwrap(); // Acquire lock
                let symbol = message.func_val.symbol.clone().unwrap();
                outputs_guard.push(message.value.clone());

                log_async(&message.call_context_id, &format!("Received output: {}", symbol));
            });

            let pair = Arc::new((Mutex::new(false), Condvar::new()));
            let pair_clone = pair.clone();

            let result_callback = Arc::new(move |result: crate::core::ExecResult<()>| {
                assert!(result.is_ok());

                let (lock, cvar) = &*pair_clone;
                let mut finished = lock.lock().unwrap(); // Acquire lock
                *finished = true; // Set the state to indicate completion
                cvar.notify_one(); // Notify the waiting thread

                let outputs_guard = o2.lock().unwrap(); // Acquire lock
                let values: Vec<IntLive> = outputs_guard.iter()
                    .map(|val| {
                        val.deref().unwrap().as_live().as_int().unwrap().unwrap()
                    })
                    .collect();
                assert_eq!(values, vec![12]);
            });

            start_call(
                api.mmu.clone(),
                1,
                main_ref,
                vec![],
                output_callback,
                result_callback,
            );

            // Wait for the result_callback to signal completion
            let (lock, cvar) = &*pair;
            let mut finished = lock.lock().unwrap();
            while !*finished {
                finished = cvar.wait(finished).unwrap();
            }
        }
    }

    #[test]
    fn test_start_call() {
        let mmu = Arc::new(MMU::new());

        {
            let mut api = GraphiteApi { mmu, symbol_table: HashMap::new() };

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
                                ["_two", "two"],
                                "two",
                                "num",
                                "doubled"
                            ],
                            "ops": [
                                ["Get", ["outer", "_two"], "two"],
                                ["Mul", ["num", "two"], "doubled"]
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
                                ["_double", "double"],
                                "double",
                                ["arg", 10],
                                "result"
                            ],
                            "ops": [
                                ["Get", ["outer", "_double"], "double"],
                                ["Call", ["double", "arg"], "result"]
                            ],
                            "input_vals": [],
                            "output_vals": ["result"]
                        }
                    }
                },
                "collections": {},
                "imports": {},
                "types": {}
            }"#;

            let collection: Collection = serde_json::from_str(json_collection).unwrap();

            api.store_collection(collection, "my_collection".to_string()).unwrap();

            let main_ref = api.get_path(vec!["my_collection".into(), "main".into()]).unwrap();

            let outputs: Arc<Mutex<Vec<ValueReference>>> = Arc::new(Mutex::new(vec![]));
            let o2 = outputs.clone();

            let output_callback = Arc::new(move |message: &NewValMessage| {
                let mut outputs_guard = outputs.lock().unwrap(); // Acquire lock
                let symbol = message.func_val.symbol.clone().unwrap();
                outputs_guard.push(message.value.clone());

                log_async(&message.call_context_id,&format!("Received output: {}", symbol));
            });

            let pair = Arc::new((Mutex::new(false), Condvar::new()));
            let pair_clone = pair.clone();

            let result_callback = Arc::new(move |result: crate::core::ExecResult<()>| {
                assert!(result.is_ok());

                let (lock, cvar) = &*pair_clone;
                let mut finished = lock.lock().unwrap(); // Acquire lock
                *finished = true; // Set the state to indicate completion
                cvar.notify_one(); // Notify the waiting thread

                let outputs_guard = o2.lock().unwrap(); // Acquire lock
                let values: Vec<IntLive> = outputs_guard.iter()
                    .map(|val| {
                        val.deref().unwrap().as_live().as_int().unwrap().unwrap()
                    })
                    .collect();
                assert_eq!(values, vec![20]);
            });

            start_call(
                api.mmu.clone(),
                2,
                main_ref,
                vec![],
                output_callback,
                result_callback,
            );

            // Wait for the result_callback to signal completion
            let (lock, cvar) = &*pair;
            let mut finished = lock.lock().unwrap();
            while !*finished {
                finished = cvar.wait(finished).unwrap();
            }
        }
    }

    #[test]
    fn test_start_call_map() {
        let mmu = Arc::new(MMU::new());

        {
            let mut api = GraphiteApi { mmu, symbol_table: HashMap::new() };

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
                                ["_two", "two"],
                                "two",
                                "num",
                                "doubled"
                            ],
                            "ops": [
                                ["Get", ["outer", "_two"], "two"],
                                ["Mul", ["num", "two"], "doubled"]
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
                                ["_double", "double"],
                                ["_my_list", "my_list"],
                                "my_list",
                                "double_list"
                            ],
                            "ops": [
                                ["Get", ["outer", "_double"], "double_func"],
                                ["Get", ["outer", "_my_list"], "my_list"],
                                ["Map", ["double_func", "my_list"], "double_list"]
                            ],
                            "input_vals": [],
                            "output_vals": ["double_list"]
                        }
                    }
                },
                "collections": {},
                "imports": {},
                "types": {}
            }"#;

            let collection: Collection = serde_json::from_str(json_collection).unwrap();

            api.store_collection(collection, "my_collection".to_string()).unwrap();

            let main_ref = api.get_path(vec!["my_collection".into(), "double_list".into()]).unwrap();

            let outputs: Arc<Mutex<Vec<ValueReference>>> = Arc::new(Mutex::new(vec![]));
            let o2 = outputs.clone();

            let output_callback = Arc::new(move |message: &NewValMessage| {
                let mut outputs_guard = outputs.lock().unwrap(); // Acquire lock
                let symbol = message.func_val.symbol.clone().unwrap();
                outputs_guard.push(message.value.clone());

                log_async(&message.call_context_id, &format!("Received output: {}", symbol));
            });

            let pair = Arc::new((Mutex::new(false), Condvar::new()));
            let pair_clone = pair.clone();

            let result_callback = Arc::new(move |result: crate::core::ExecResult<()>| {
                assert!(result.is_ok());

                let (lock, cvar) = &*pair_clone;
                let mut finished = lock.lock().unwrap(); // Acquire lock
                *finished = true; // Set the state to indicate completion
                cvar.notify_one(); // Notify the waiting thread

                let outputs_guard = o2.lock().unwrap(); // Acquire lock
                let values: Vec<IntLive> = outputs_guard.iter()
                    .map(|val| {
                        // val.deref().unwrap().as_live().as_int().unwrap().unwrap()

                        match val.deref() {
                            Ok(d) => d,
                            Err(e) => panic!("Error dereferencing result: {}", e)
                        };

                        val.deref().unwrap().as_live().as_int().unwrap().unwrap()

                    })
                    .collect();
                assert_eq!(values, vec![20, 40, 60]);
            });

            start_call(
                api.mmu.clone(),
                2,
                main_ref,
                vec![],
                output_callback,
                result_callback,
            );

            // Wait for the result_callback to signal completion
            let (lock, cvar) = &*pair;
            let mut finished = lock.lock().unwrap();
            while !*finished {
                finished = cvar.wait(finished).unwrap();
            }
        }

    }

}