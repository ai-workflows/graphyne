use std::sync::{Arc, mpsc};
use std::thread;
use rayon::ThreadPool;
use crate::core::vm::functions::v2::{executor, orchestrator};
use crate::core::vm::functions::v2::shared::{NewOpMessage, NewValMessage, SharedCallState};
use crate::core::vm::VM;

pub fn start_call<'a>(
    vm: Arc<VM>,
    max_workers: usize,
) {
    let (new_op_sender, new_op_receiver) = mpsc::channel::<NewOpMessage>();
    let (new_val_sender, new_val_receiver) = mpsc::channel::<NewValMessage>();

    let shared_state = SharedCallState::new(
        vm,
        new_op_sender,
        new_val_sender,
    );
    let ss2 = shared_state.clone();

    let or_pool: ThreadPool = rayon::ThreadPoolBuilder::new().num_threads(max_workers).build().unwrap();
    // Orchestrator Dispatcher thread
    thread::spawn(move || {
        for message in new_val_receiver.iter() {
            let ss = shared_state.clone();

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
                        ss.handle_error(&message.call_context_id, format!("Orchestrator encountered an error: {}", e))
                    }
                }
            });
        }
    });

    let ex_pool: ThreadPool = rayon::ThreadPoolBuilder::new().num_threads(max_workers).build().unwrap();
    // Executor Dispatcher thread
    thread::spawn(move || {
        for message in new_op_receiver.iter() {
            let ss = ss2.clone();

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
                        ss.handle_error(&message.call_context_id, format!("Executor encountered an error: {}", e))
                    }
                }
            });
        }
    });
}