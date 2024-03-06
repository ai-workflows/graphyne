use std::collections::{HashMap, HashSet};
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Sender};
use rayon::ThreadPool;
use crate::runtime::data::functions::val::FuncValId;
use crate::runtime::data::live::{FuncValLive};
use crate::runtime::{ExecResult, Symbol};
use crate::runtime::data::stored::lists::ptrs_to_func_val_list;
use crate::runtime::data::stored::StoredData;
use crate::runtime::static_state::state::StaticState;
use crate::runtime::vm::{executor, orchestrator};
use crate::runtime::vm::orchestrator::handle_called_fn_constants;
use crate::runtime::vm::shared::{CallContextId, ExecutorMessage, NewOpMessage, OrchestratorMessage, send_new_val, SharedCallState};



pub enum StreamResult {
    NumOutputs(usize),
    Output(FuncValLive, Arc<StoredData>),
    Error(String)
}

pub fn manage_await_call(
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>,
    func: Arc<StoredData>,
    args: Vec<Arc<StoredData>>,
    verbose: bool,
) -> ExecResult<HashMap<Symbol, Arc<StoredData>>> {
    let (outputs_sender, outputs_receiver) = mpsc::channel::<StreamResult>();

    manage_start_call(
        static_state.clone(),
        worker_pool.clone(),
        func,
        args,
        Arc::new(Mutex::new(outputs_sender)),
        verbose,
    ).unwrap();

    let mut outputs: HashMap<Symbol, Arc<StoredData>> = HashMap::new();
    let mut output_count: Option<usize> = None;

    for res in outputs_receiver.iter() {
        match res {
            StreamResult::NumOutputs(num) => {
                output_count = Some(num);
            },
            StreamResult::Output(fn_val, val_ref) => {
                outputs.insert(fn_val.guid.clone(), val_ref);
            },
            StreamResult::Error(e) => {
                return Err(e);
            }
        }

        if let Some(output_count) = output_count {
            if outputs.len() == output_count {
                break;
            }
        }
    }

    Ok(outputs)

}


pub fn manage_start_call<'a>(
    static_state: Arc<StaticState>,
    worker_pool: Arc<ThreadPool>,
    func: Arc<StoredData>,
    args: Vec<Arc<StoredData>>,
    output_sender: Arc<Mutex<Sender<StreamResult>>>,
    verbose: bool,

) -> ExecResult<()> {
    // generate a random call context id
    let main_call_id = uuid::Uuid::new_v4().to_string();

    // get the function's outputs
    let func_live = func.stored_as_func()?;
    let output_fn_vals = ptrs_to_func_val_list(&func_live.output_vals)?;

    let final_outputs: HashSet<(CallContextId, FuncValId)> = output_fn_vals.iter()
        .map(|val| (main_call_id.clone(), val.guid.clone()))
        .collect();

    let shared_state: Arc<SharedCallState> = SharedCallState::new(
        static_state.clone(),
        output_sender.clone(),
        final_outputs,
        worker_pool.clone(),
        verbose,
    );

    shared_state.log_async(&main_call_id, &"Starting new call".to_string());

    // get the function's inputs
    let input_fn_vals = ptrs_to_func_val_list(&func_live.input_vals)?;

    // match the function's inputs with the provided args
    let input_fn_vals: Vec<(Arc<StoredData>, FuncValLive)> = args.iter()
        .zip(input_fn_vals)
        .map(|(arg, val)| (arg.clone(), val.clone()))
        .collect();

    // send the inputs as new values
    for (val_ref, func_val) in input_fn_vals {
        send_new_val(shared_state.clone(), &main_call_id, &func_val, val_ref);
    }

    // send the function's constants as new values
    match handle_called_fn_constants(shared_state.clone(), &main_call_id, &func_live) {
        Ok(_) => {},
        Err(e) => {
            // shared_state.halt_execution(&main_call_id, CallResult::Error(e.clone()));
            return Err(e);
        }
    }

    // send an output message with the number of expected outputs
    output_sender.lock().unwrap().send(StreamResult::NumOutputs(output_fn_vals.len())).unwrap();

    // manage_call(shared_state, output_sender, &func_live)

    Ok(())
}

// fn manage_call(
//     shared_state: Arc<SharedCallState>,
//     control_receiver: Receiver<ControlMessage>,
//     output_sender: Option<Arc<Mutex<Sender<StreamResult>>>>,
//     main_func: &FuncLive
// ) -> ExecResult<Vec<Arc<StoredData>>> {
//     let outputs: Arc<Mutex<Vec<(FuncValLive, Arc<StoredData>)>>> = Arc::new(Mutex::new(vec![]));
//
//     for message in control_receiver.iter() {
//         let ss = shared_state.clone();
//
//         let status: CallStatus = match message {
//             ControlMessage::FromExecutor(msg) => manage_executor_result(msg, ss, output_sender.clone()),
//             ControlMessage::FromOrchestrator(msg) => manage_orchestrator_result(msg, ss),
//             ControlMessage::Error(_, e) => {
//                 if let Some(output_sender) = output_sender {
//                     output_sender.lock().unwrap().send(StreamResult::Error(e.clone())).unwrap();
//                 }
//                 return Err(e);
//             }
//         };
//
//         match status {
//             CallStatus::Success => break,
//             CallStatus::Error(e) => {
//                 if let Some(output_sender) = output_sender {
//                     output_sender.lock().unwrap().send(StreamResult::Error(e.clone())).unwrap();
//                 }
//                 return Err(e);
//             },
//             CallStatus::Pending => {}
//         }
//     }
//
//     // get the outputs in the same order as the function's output values
//     let outputs = outputs.lock().unwrap();
//     let fn_output_vals = get_func_vals_from_ptrs(shared_state.mmu.clone(), &main_func.output_vals).unwrap();
//     let outputs: Vec<Arc<StoredData>> = fn_output_vals.iter()
//         .map(|val| {
//             let output = outputs.iter()
//                 .find(|(output_fn_val, _)| output_fn_val.guid == val.guid)
//                 .unwrap();
//             output.1.clone()
//         })
//         .collect();
//
//     Ok(outputs)
// }

pub enum CallStatus {
    Success,
    Pending,
    Error(String)
}

pub fn manage_executor_result(
    message: ExecutorMessage,
    ss: Arc<SharedCallState>,
) {
    match message {
        ExecutorMessage::NewVal(res) => {
            // if the value is a final output, send it to the output sender
            if ss.check_for_final_output(&res.call_context_id, &res.func_val) {
                ss.send_output(res.func_val.clone(), res.value.clone());

                // if there are no remaining final outputs, halt execution
                // if !ss.has_remaining_final_outputs() {
                //     return CallStatus::Success;
                // }
            }

            let ss2 = ss.clone();
            ss.worker_pool.spawn(move || {
                match orchestrator::handle_new_value_v2(
                    ss2.clone(),
                    &res.call_context_id,
                    &res.func_val,
                    res.value,
                ) {
                    Ok(_) => {},
                    Err(e) => {
                        // if an error occurred, throw an error control message
                        ss2.throw_error(
                            &res.call_context_id,
                            &format!("Orchestrator encountered an error: {}", e)
                        );
                    }
                }
            });
        },
        ExecutorMessage::Pending(res) => {
            // expect the thread that sends the new value to remove the op from the pending ops
            ss.log_async(&res.call_context_id, &format!("Value calculation pending: {}", res.func_val.symbol.unwrap()));
        }
    }
}

pub fn manage_orchestrator_result(
    message: OrchestratorMessage,
    ss: Arc<SharedCallState>,
) {
    let message: NewOpMessage = match message {
        OrchestratorMessage::NewOp(msg) => msg
    };

    let ss2 = ss.clone();
    ss.worker_pool.spawn(move || {
        match executor::try_execute_fn_op(ss2.clone(), &message.op, &message.call_context_id) {
            Ok(results) => {
                // if successful, send the results to the state manager
                for ex_message in results {
                    match ex_message {
                        ExecutorMessage::NewVal(result) => {
                            // remove the op from the pending ops
                            ss2.complete_pending_op(&message.call_context_id, &message.op.guid);

                            send_new_val(ss2.clone(), &result.call_context_id, &result.func_val, result.value);
                        },
                        ExecutorMessage::Pending(result) => {
                            // expect the thread that sends the new value to remove the op from the pending ops
                            ss2.log_async(&result.call_context_id, &format!("Value calculation pending: {}", result.func_val.symbol.unwrap()));
                        }
                    }
                }
            },
            Err(e) => {
                // remove the op from the pending ops
                ss2.complete_pending_op(&message.call_context_id, &message.op.guid);

                // if an error occurred, handle it by halt execution
                ss2.throw_error(
                    &message.call_context_id,
                    &format!("Executor encountered an error: {}", e)
                );
            }
        }
    });
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, mpsc};
    use crate::binder::intermediate::collection::Collection;
    use crate::binder::Binder;
    use crate::runtime::data::live::{LiveData, IntLive};
    use crate::runtime::mmu::mmu::MMU;
    use crate::runtime::vm::manager::{manage_await_call, manage_start_call, StreamResult};

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
                                ["Get", ["outer", "_two"], ["two"]],
                                ["Add", ["num", "two"], ["result"]]
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

            let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());

            let start_time = std::time::Instant::now();
            for _ in 0..1000 {
                let _ = manage_await_call(
                    api.mmu.clone(),
                    worker_pool.clone(),
                    main_ref.clone(),
                    vec![],
                    false,
                );
            }
            let elapsed = start_time.elapsed();
            println!("Average time: {:?}", elapsed / 1000);

            let res = manage_await_call(
                api.mmu.clone(),
                worker_pool,
                main_ref,
                vec![],
                true,
            );

            let outputs = match res {
                Ok(outputs) => outputs,
                Err(e) => panic!("Call returned an error: {}", e)
            };

            assert_eq!(outputs.len(), 1);

            let output_values: Vec<Arc<StoredData>> = outputs.values().cloned().collect();

            let result = api.mmu.get_ref_value(&output_values[0]).unwrap().as_live().as_int().unwrap().unwrap();

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
                                ["Get", ["outer", "_two"], ["two"]],
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
                                ["_double", "double"],
                                "double",
                                ["arg", 10],
                                "result"
                            ],
                            "ops": [
                                ["Get", ["outer", "_double"], ["double"]],
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

            api.store_collection(collection, "my_collection".to_string()).unwrap();

            let main_ref = api.get_path(vec!["my_collection".into(), "main".into()]).unwrap();

            let (outputs_sender, _) = mpsc::channel::<StreamResult>();

            let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap());

            let res = manage_await_call(
                api.mmu.clone(),
                worker_pool,
                main_ref,
                vec![],
                true,
            ).unwrap();

            // let mut outputs: Vec<(FuncValLive, Arc<StoredData>)> = vec![];
            //
            // for i in 0..expected_num_outputs {
            //     let res = outputs_receiver.recv().unwrap();
            //     match res {
            //         StreamResult::Output(fn_val, val_ref) => {
            //             outputs.push((fn_val, val_ref));
            //         },
            //         StreamResult::Error(e) => {
            //             panic!("Call returned an error: {}", e);
            //         }
            //     }
            // }

            // let value: IntLive = api.mmu.get_ref_value(&res.get(0).unwrap()).unwrap().as_live().as_int().unwrap().unwrap();

            assert_eq!(res.len(), 1);

            let output_values: Vec<Arc<StoredData>> = res.values().cloned().collect();

            let value = api.mmu.get_ref_value(&output_values[0]).unwrap().as_live().as_int().unwrap().unwrap();

            assert_eq!(value, 20);
        }
    }

    #[test]
    fn test_reduce() {
        let mmu: Arc<MMU> = Arc::new(MMU::new());

        {
            let mut api = Binder { mmu, symbol_table: HashMap::new() };

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
                                ["_add", "add"],
                                "add",
                                ["_my_list", "my_list"],
                                "my_list",
                                ["initial", 0],
                                "result"
                            ],
                            "ops": [
                                ["Get", ["outer", "_add"], ["add"]],
                                ["Get", ["outer", "_my_list"], ["my_list"]],
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

            api.store_collection(collection, "my_collection".to_string()).unwrap();

            let main_ref = api.get_path(vec!["my_collection".into(), "main".into()]).unwrap();

            let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());

            let start_time = std::time::Instant::now();

            for _ in 0..1000 {
                let _ = manage_await_call(
                    api.mmu.clone(),
                    worker_pool.clone(),
                    main_ref.clone(),
                    vec![],
                    false,
                );
            }

            let elapsed = start_time.elapsed();
            println!("Average time: {:?}", elapsed / 1000);

            let res = manage_await_call(
                api.mmu.clone(),
                worker_pool,
                main_ref,
                vec![],
                true,
            );

            let outputs = match res {
                Ok(outputs) => outputs,
                Err(e) => panic!("Call returned an error: {}", e)
            };

            assert_eq!(outputs.len(), 1);

            // let result = outputs[0].deref().unwrap().as_live().as_int().unwrap().unwrap();

            let output_values: Vec<Arc<StoredData>> = outputs.values().cloned().collect();
            assert_eq!(output_values.len(), 1);
            let result = api.mmu.get_ref_value(&output_values[0]).unwrap().as_live().as_int().unwrap().unwrap();

            assert_eq!(result, 1200);
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
                                ["Get", ["outer", "_two"], ["two"]],
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
                                ["_double", "double"],
                                ["_my_list", "my_list"],
                                "my_list",
                                "double_list"
                            ],
                            "ops": [
                                ["Get", ["outer", "_double"], ["double_func"]],
                                ["Get", ["outer", "_my_list"], ["my_list"]],
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

            api.store_collection(collection, "my_collection".to_string()).unwrap();

            let main_ref = api.get_path(vec!["my_collection".into(), "double_list".into()]).unwrap();

            let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap());

            let res = manage_await_call(
                api.mmu.clone(),
                worker_pool,
                main_ref,
                vec![],
                true,
            );

            let outputs = match res {
                Ok(outputs) => outputs,
                Err(e) => panic!("Call returned an error: {}", e)
            };

            assert_eq!(outputs.len(), 1);

            let output_values: Vec<Arc<StoredData>> = outputs.values().cloned().collect();
            assert_eq!(output_values.len(), 1);

            let result = api.mmu.get_ref_value(&output_values[0]).unwrap().as_live().as_list().unwrap().unwrap();

            // let result = outputs[0].deref().unwrap().as_live().as_list().unwrap().unwrap();
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
                            ["_is_even", "is_even"],
                            ["_my_list", "my_list"],
                            "my_list",
                            "even_list"
                        ],
                        "ops": [
                            ["Get", ["outer", "_is_even"], ["is_even"]],
                            ["Get", ["outer", "_my_list"], ["my_list"]],
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

        binder.store_collection(collection, "my_collection".to_string()).unwrap();

        let main_ref = binder.get_path(vec!["my_collection".into(), "filter_even".into()]).unwrap();

        let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap());

        let start_time = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = manage_await_call(
                binder.mmu.clone(),
                worker_pool.clone(),
                main_ref.clone(),
                vec![],
                false,
            );
        }
        let elapsed = start_time.elapsed();
        println!("1000 calls took: {:?}", elapsed);

        let res = manage_await_call(
            binder.mmu.clone(),
            worker_pool,
            main_ref,
            vec![],
            true,
        );

        let outputs = match res {
            Ok(outputs) => outputs,
            Err(e) => panic!("Call returned an error: {}", e)
        };

        assert_eq!(outputs.len(), 1);

        let output_values: Vec<Arc<StoredData>> = outputs.values().cloned().collect();
        assert_eq!(output_values.len(), 1);

        let result = binder.mmu.get_ref_value(&output_values[0]).unwrap().as_live().as_list().unwrap().unwrap();

        // let result = outputs[0].deref().unwrap().as_live().as_list().unwrap().unwrap();
        let result: Vec<IntLive> = result.iter().map(|ptr|
            binder.mmu.get_ptr_value(ptr).unwrap().as_live().as_int().unwrap().unwrap()).collect();

        assert_eq!(result, vec![2, 4, 6, 8, 10]);
    }
}