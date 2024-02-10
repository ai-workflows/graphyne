use std::collections::HashSet;
use std::sync::{Arc, mpsc};
use std::thread;
use rayon::ThreadPool;
use crate::core::data::functions::val::FuncValId;
use crate::core::data::live::FuncValLive;
use crate::core::ExecResult;
use crate::core::vm::mmu::mmu::{MMU};
use crate::core::vm::v2::{executor, orchestrator};
use crate::core::vm::v2::orchestrator::handle_called_fn_constants;
use crate::core::vm::v2::shared::{CallContextId, ExecutorMessage, get_func_from_ptr, get_func_vals_from_ptrs, log_async, NewOpMessage, NewValMessage, SharedCallState};
use crate::core::vm::value_ref::ValueReference;

pub fn start_call<'a>(
    mmu: Arc<MMU>,
    ex_pool: Arc<ThreadPool>,
    or_pool: Arc<ThreadPool>,
    func: ValueReference,
    args: Vec<ValueReference>,
    output_callback: Arc<impl Fn(&NewValMessage) + Send + Sync + 'static>,
    result_callback: Arc<impl Fn(ExecResult<()>) + Send + Sync + 'static>,
) {
    // generate a random call context id
    let main_call_id = uuid::Uuid::new_v4().to_string();
    log_async(&main_call_id, &"Starting new call".to_string());

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
        ex_pool.clone(),
        or_pool.clone(),
    );

    // start the orchestrator and executor threads
    start_orchestrator(shared_state.clone(), new_val_receiver, output_callback, result_callback.clone());
    start_executor(shared_state.clone(), new_op_receiver, result_callback.clone());

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

fn start_orchestrator(
    shared_state: Arc<SharedCallState>,
    new_val_receiver: mpsc::Receiver<NewValMessage>,
    output_callback: Arc<impl Fn(&NewValMessage) + Send + Sync + 'static>,
    result_callback: Arc<impl Fn(ExecResult<()>) + Send + Sync + 'static>,
) {
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

            let or_pool = ss.orchestrator_thread_pool.clone();
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

fn start_executor(
    shared_state: Arc<SharedCallState>,
    new_op_receiver: mpsc::Receiver<NewOpMessage>,
    result_callback: Arc<impl Fn(ExecResult<()>) + Send + Sync + 'static>,
) {
    // Executor Dispatcher thread
    thread::spawn(move || {
        for message in new_op_receiver.iter() {
            let ss = shared_state.clone();
            let result_callback = result_callback.clone();
            let ex_pool = ss.executor_thread_pool.clone();

            ex_pool.spawn(move || {
                match executor::try_execute_fn_op(ss.clone(), &message.op, &message.call_context_id) {
                    Ok(results) => {
                        // if successful, send the results to the state manager
                        for ex_message in results {
                            match ex_message {
                                ExecutorMessage::NewVal(message) => {
                                    ss.send_new_val(message.call_context_id.clone(), message.func_val, message.value);
                                },
                                ExecutorMessage::Pending(message) => {
                                    log_async(&message.call_context_id, &format!("Value calculation pending: {}", message.func_val.symbol.unwrap()));
                                }
                            }
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

/// starts a call, waits for it to complete, and returns the results
pub fn await_call(
    mmu: Arc<MMU>,
    ex_pool: Arc<ThreadPool>,
    or_pool: Arc<ThreadPool>,
    func: ValueReference,
    args: Vec<ValueReference>,
) -> ExecResult<Vec<ValueReference>> {
    let (tx, rx) = mpsc::channel();
    let shared_tx = Arc::new(tx);
    let shared_tx2 = shared_tx.clone();

    let func_live = get_func_from_ptr(mmu.clone(), &func.pointer).unwrap();
    let expected_output_count = func_live.output_vals.len();

    let output_callback = Arc::new(move |message: &NewValMessage| {
        let tx2 = shared_tx.clone();

        // collect outputs
        let mut outputs = Vec::new();
        outputs.push(message.value.clone());

        // Send only if we've collected all possible outputs
        if outputs.len() == expected_output_count {
            tx2.send(Ok(outputs)).unwrap();
        }
    });

    let result_callback = Arc::new(move |result: ExecResult<()>| {
        let tx2 = shared_tx2.clone();

        if let Err(e) = result {
            tx2.send(Err(e)).unwrap();
        }
    });

    start_call(
        mmu,
        ex_pool,
        or_pool,
        func,
        args,
        output_callback,
        result_callback,
    );

    rx.recv().unwrap()
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Condvar, Mutex};
    use crate::api::collections::collection::Collection;
    use crate::api::GraphiteApi;
    use crate::core::data::live::{LiveData, IntLive};
    use crate::core::vm::mmu::mmu::MMU;
    use crate::core::vm::v2::manager::{await_call, start_call};
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

            let ex_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());
            let or_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());

            let res = await_call(
                api.mmu.clone(),
                ex_pool,
                or_pool,
                main_ref,
                vec![],
            );

            let outputs = match res {
                Ok(outputs) => outputs,
                Err(e) => panic!("Call returned an error: {}", e)
            };

            assert_eq!(outputs.len(), 1);

            let result = outputs[0].deref().unwrap().as_live().as_int().unwrap().unwrap();

            assert_eq!(result, 12);
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

            let ex_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());
            let or_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());

            start_call(
                api.mmu.clone(),
                ex_pool,
                or_pool,
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
    fn test_reduce() {
        let mmu: Arc<MMU> = Arc::new(MMU::new());

        {
            let mut api = GraphiteApi { mmu, symbol_table: HashMap::new() };

            let json_collection = r#"{
                "constants": {
                    "my_list": [10, 20, 30]
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
                                ["Add", ["num1", "num2"], "sum"]
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
                                ["_add", "add"],
                                "add",
                                ["_my_list", "my_list"],
                                "my_list",
                                ["initial", 0],
                                "result"
                            ],
                            "ops": [
                                ["Get", ["outer", "_add"], "add"],
                                ["Get", ["outer", "_my_list"], "my_list"],
                                ["Reduce", ["add", "my_list", "initial"], "result"]
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

            let ex_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());
            let or_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());

            let res = await_call(
                api.mmu.clone(),
                ex_pool,
                or_pool,
                main_ref,
                vec![],
            );

            let outputs = match res {
                Ok(outputs) => outputs,
                Err(e) => panic!("Call returned an error: {}", e)
            };

            assert_eq!(outputs.len(), 1);

            let result = outputs[0].deref().unwrap().as_live().as_int().unwrap().unwrap();

            assert_eq!(result, 60);
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

            let ex_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());
            let or_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());

            let res = await_call(
                api.mmu.clone(),
                ex_pool,
                or_pool,
                main_ref,
                vec![],
            );

            let outputs = match res {
                Ok(outputs) => outputs,
                Err(e) => panic!("Call returned an error: {}", e)
            };

            assert_eq!(outputs.len(), 1);

            let result = outputs[0].deref().unwrap().as_live().as_list().unwrap().unwrap();
            let result: Vec<IntLive> = result.iter().map(|ptr|
                api.mmu.get_ptr_value(ptr).unwrap().as_live().as_int().unwrap().unwrap()).collect();

            assert_eq!(result, vec![20, 40, 60]);
        }

    }

}