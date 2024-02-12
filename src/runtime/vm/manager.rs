use std::collections::HashSet;
use std::sync::{Arc, mpsc, RwLock};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use rayon::ThreadPool;
use crate::runtime::data::functions::val::FuncValId;
use crate::runtime::data::live::FuncValLive;
use crate::runtime::ExecResult;
use crate::runtime::mmu::mmu::MMU;
use crate::runtime::mmu::value_ref::ValueReference;
use crate::runtime::vm::{executor, orchestrator};
use crate::runtime::vm::orchestrator::handle_called_fn_constants;
use crate::runtime::vm::shared::{CallContextId, CallResult, ExecutorMessage, get_func_from_ptr, get_func_vals_from_ptrs, NewOpMessage, NewValMessage, SharedCallState};


/// starts a call, waits for it to complete, and returns the results
pub fn manage_await_call(
    mmu: Arc<MMU>,
    ex_pool: Arc<ThreadPool>,
    or_pool: Arc<ThreadPool>,
    func: ValueReference,
    args: Vec<ValueReference>,
    verbose: bool,
) -> ExecResult<Vec<ValueReference>> {
    let func_live = get_func_from_ptr(mmu.clone(), &func.pointer).unwrap();

    let outputs: Arc<RwLock<Vec<(ValueReference, FuncValLive)>>>  = Arc::new(RwLock::new(
        Vec::with_capacity(func_live.output_vals.len())
    ));
    let o2 = outputs.clone();

    let output_callback = Arc::new(move |message: &NewValMessage| {
        let mut outputs_guard = o2.write().unwrap(); // Acquire lock
        outputs_guard.push((message.value.clone(), message.func_val.clone()));
    });

    let result_receiver = manage_start_call(
        mmu.clone(),
        ex_pool,
        or_pool,
        func,
        args,
        output_callback,
        verbose,
        None
    );

    match result_receiver.recv() {
        Ok(v) => match v {
            CallResult::Success => (),
            CallResult::Error(e) => return Err(e)
        }
        Err(e) => return Err(format!("Error receiving call result: {}", e))
    };

    // get the outputs in the same order as the function's output values
    let fn_output_vals = get_func_vals_from_ptrs(mmu.clone(), &func_live.output_vals).unwrap();

    let outputs_guard = outputs.read().unwrap(); // Acquire lock
    let outputs: Vec<ValueReference> = fn_output_vals.iter()
        .map(|val| {
            let output = outputs_guard.iter()
                .find(|(_, output_fn_val)| output_fn_val.guid == val.guid)
                .unwrap();
            output.0.clone()
        })
        .collect();

    Ok(outputs)
}

pub fn manage_start_call<'a>(
    mmu: Arc<MMU>,
    ex_pool: Arc<ThreadPool>,
    or_pool: Arc<ThreadPool>,
    func: ValueReference,
    args: Vec<ValueReference>,
    output_callback: Arc<impl Fn(&NewValMessage) + Send + Sync + 'static>,
    verbose: bool,
    results_channel: Option<(Sender<CallResult>, Receiver<CallResult>)> // allows the caller to provide a custom results channel

) -> Receiver<CallResult> {
    let (results_sender, results_receiver) = match results_channel {
        Some((sender, receiver)) => (sender, receiver),
        None => mpsc::channel::<CallResult>()
    };

    // generate a random call context id
    let main_call_id = uuid::Uuid::new_v4().to_string();

    // get the function's outputs
    let func_live = get_func_from_ptr(mmu.clone(), &func.pointer).unwrap();
    let output_fn_vals = match get_func_vals_from_ptrs(
        mmu.clone(),
        &func_live.output_vals,
    ) {
        Ok(v) => v,
        Err(e) => {
            let error_msg = format!("Error getting function outputs: {}", e);
            results_sender.send(CallResult::Error(error_msg)).unwrap();
            return results_receiver;
        }
    };

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
        results_sender,
        final_outputs,
        ex_pool.clone(),
        or_pool.clone(),
        verbose,
    );

    shared_state.log_async(&main_call_id, &"Starting new call".to_string());

    // start the orchestrator and executor threads
    start_orchestrator(shared_state.clone(), new_val_receiver, output_callback);
    start_executor(shared_state.clone(), new_op_receiver);

    // get the function's inputs
    let input_fn_vals = match get_func_vals_from_ptrs(
        mmu.clone(),
        &func_live.input_vals,
    ) {
        Ok(v) => v,
        Err(e) => {
            shared_state.halt_execution(&main_call_id, CallResult::Error(e));
            return results_receiver;
        }
    };

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
    match handle_called_fn_constants(shared_state.clone(), &main_call_id, &func_live) {
        Ok(_) => {},
        Err(e) => {
            shared_state.halt_execution(&main_call_id, CallResult::Error(e));
            return results_receiver;
        }
    }

    results_receiver
}

fn start_orchestrator(
    shared_state: Arc<SharedCallState>,
    new_val_receiver: Receiver<NewValMessage>,
    output_callback: Arc<impl Fn(&NewValMessage) + Send + Sync + 'static>
) {
    // Orchestrator Dispatcher thread
    thread::spawn(move || {
        for message in new_val_receiver.iter() {
            let ss = shared_state.clone();

            if ss.is_halted(){
                return;
            }

            // call the output callback if the message is a final output
            if ss.check_for_final_output(&message.call_context_id, &message.func_val) {
                output_callback(&message);
            }

            // if there are no remaining final outputs, halt execution
            if !ss.has_remaining_final_outputs() {
                ss.halt_execution(&message.call_context_id, CallResult::Success);
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
                        ss.halt_execution(
                            &message.call_context_id,
                            CallResult::Error(error_msg.clone())
                        );
                    }
                }
            });
        }
    });
}

fn start_executor(
    shared_state: Arc<SharedCallState>,
    new_op_receiver: Receiver<NewOpMessage>
) {
    // Executor Dispatcher thread
    thread::spawn(move || {
        for message in new_op_receiver.iter() {
            let mut ss = shared_state.clone();
            let ex_pool = ss.executor_thread_pool.clone();

            if ss.is_halted(){
                return;
            }

            ex_pool.spawn(move || {
                match executor::try_execute_fn_op(ss.clone(), &message.op, &message.call_context_id) {
                    Ok(results) => {
                        // if successful, send the results to the state manager
                        for ex_message in results {
                            match ex_message {
                                ExecutorMessage::NewVal(result) => {
                                    // remove the op from the pending ops
                                    ss.complete_pending_op(&message.call_context_id, &message.op.guid);

                                    ss.send_new_val(result.call_context_id.clone(), result.func_val, result.value);
                                },
                                ExecutorMessage::Pending(result) => {
                                    // expect the thread that sends the new value to remove the op from the pending ops
                                    ss.log_async(&result.call_context_id, &format!("Value calculation pending: {}", result.func_val.symbol.unwrap()));
                                }
                            }
                        }
                    },
                    Err(e) => {
                        // remove the op from the pending ops
                        ss.complete_pending_op(&message.call_context_id, &message.op.guid);

                        // if an error occurred, handle it by halt execution
                        let error_msg = format!("Executor encountered an error: {}", e);
                        ss.halt_execution(
                            &message.call_context_id,
                            CallResult::Error(error_msg.clone())
                        );
                    }
                }
            });
        }
    });
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io;
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use crate::binder::intermediate::collection::Collection;
    use crate::binder::Binder;
    use crate::runtime::data::live::{LiveData, IntLive};
    use crate::runtime::mmu::mmu::MMU;
    use crate::runtime::mmu::value_ref::ValueReference;
    use crate::runtime::vm::manager::{manage_await_call, manage_start_call};
    use crate::runtime::vm::shared::{NewValMessage};

    #[test]
    fn test_start_call_simple() {
        let mmu: Arc<MMU> = Arc::new(MMU::new());

        {
            let mut api = Binder { mmu, symbol_table: HashMap::new() };

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

            let res = manage_await_call(
                api.mmu.clone(),
                ex_pool,
                or_pool,
                main_ref,
                vec![],
                true,
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
            let mut api = Binder { mmu, symbol_table: HashMap::new() };

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
                "intermediate": {},
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

                let stdout = io::stdout();
                let _ = writeln!(&mut stdout.lock(),
                                 "[{}] {}",
                                 message.call_context_id,
                                 format!("Received output: {}", symbol)
                );
            });

            let ex_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());
            let or_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());

            manage_start_call(
                api.mmu.clone(),
                ex_pool,
                or_pool,
                main_ref,
                vec![],
                output_callback,
                true,
                None
            ).recv().unwrap();

            let outputs_guard = o2.lock().unwrap(); // Acquire lock
            let values: Vec<IntLive> = outputs_guard.iter()
                .map(|val| {
                    val.deref().unwrap().as_live().as_int().unwrap().unwrap()
                })
                .collect();

            assert_eq!(values, vec![20]);
        }
    }

    #[test]
    fn test_reduce() {
        let mmu: Arc<MMU> = Arc::new(MMU::new());

        {
            let mut api = Binder { mmu, symbol_table: HashMap::new() };

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
                "intermediate": {},
                "imports": {},
                "types": {}
            }"#;

            let collection: Collection = serde_json::from_str(json_collection).unwrap();

            api.store_collection(collection, "my_collection".to_string()).unwrap();

            let main_ref = api.get_path(vec!["my_collection".into(), "main".into()]).unwrap();

            let ex_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());
            let or_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());

            let res = manage_await_call(
                api.mmu.clone(),
                ex_pool,
                or_pool,
                main_ref,
                vec![],
                true,
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
            let mut api = Binder { mmu, symbol_table: HashMap::new() };

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
                "intermediate": {},
                "imports": {},
                "types": {}
            }"#;

            let collection: Collection = serde_json::from_str(json_collection).unwrap();

            api.store_collection(collection, "my_collection".to_string()).unwrap();

            let main_ref = api.get_path(vec!["my_collection".into(), "double_list".into()]).unwrap();

            let ex_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());
            let or_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());

            let res = manage_await_call(
                api.mmu.clone(),
                ex_pool,
                or_pool,
                main_ref,
                vec![],
                true,
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

    #[test]
    fn test_filter() {
        let mmu: Arc<MMU> = Arc::new(MMU::new());

        let mut binder = Binder { mmu, symbol_table: HashMap::new() };

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
                            ["Mod", ["num", "two"], "mod_result"],
                            ["Equal", ["mod_result", "zero"], "is_even"]
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
                            ["_is_even", "is_even"],
                            ["_my_list", "my_list"],
                            "my_list",
                            "even_list"
                        ],
                        "ops": [
                            ["Get", ["outer", "_is_even"], "is_even"],
                            ["Get", ["outer", "_my_list"], "my_list"],
                            ["Filter", ["is_even", "my_list"], "even_list"]
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

        binder.store_collection(collection, "my_collection".to_string()).unwrap();

        let main_ref = binder.get_path(vec!["my_collection".into(), "filter_even".into()]).unwrap();

        let ex_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());
        let or_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap());

        let res = manage_await_call(
            binder.mmu.clone(),
            ex_pool,
            or_pool,
            main_ref,
            vec![],
            true,
        );

        let outputs = match res {
            Ok(outputs) => outputs,
            Err(e) => panic!("Call returned an error: {}", e)
        };

        assert_eq!(outputs.len(), 1);

        let result = outputs[0].deref().unwrap().as_live().as_list().unwrap().unwrap();
        let result: Vec<IntLive> = result.iter().map(|ptr|
            binder.mmu.get_ptr_value(ptr).unwrap().as_live().as_int().unwrap().unwrap()).collect();

        assert_eq!(result, vec![2, 4, 6, 8, 10]);
    }

}