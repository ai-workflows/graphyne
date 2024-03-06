use std::collections::HashMap;
use std::{fs, io};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use crate::binder::binder;
use crate::binder::intermediate::collection::Collection;
use crate::binder::json::jsonify;
use crate::runtime::data::live::{FuncValLive, PointerLive};
use crate::runtime::{ExecResult, Symbol};
use crate::runtime::static_state::state::StaticState;
use crate::runtime::vm::manager::{manage_await_call, manage_start_call, StreamResult};

pub fn await_call(
    func: PointerLive,
    args: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    verbose: bool,
    workers: Option<usize>,
) -> ExecResult<HashMap<Symbol, PointerLive>> {
    let worker_count = get_worker_counts(workers);
    let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(worker_count).build().unwrap());

    manage_await_call(
        static_state.clone(),
        worker_pool,
        func,
        args,
        verbose,
    )
}

pub fn stream_call(
    func: PointerLive,
    args: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    verbose: bool,
    workers: Option<usize>,
    output_sender: Sender<StreamResult>,
) -> ExecResult<()> {
    let worker_count = get_worker_counts(workers);
    let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(worker_count).build().unwrap());

    let output_sender = Arc::new(Mutex::new(output_sender));

    manage_start_call(
        static_state.clone(),
        worker_pool,
        func,
        args,
        output_sender,
        verbose,
    )
}

/// Loads a Graphite JSON Intermediate Language (GJIL) file from the given path and binds to memory.
pub fn load_intermediate(path: &str) -> Result<Collection, String> {
    let contents = fs::read_to_string(path).map_err(|e| format!("Error reading intermediate file: {}", e))?;
    let program: Collection = serde_json::from_str(&contents).map_err(|e| format!("Error parsing intermediate JSON: {}", e))?;

    Ok(program)
}

pub fn bind(program: Collection, program_symbol: Option<Symbol>) -> Result<Arc<StaticState>, String> {
    let mut static_state = StaticState::new();

    binder::bind_program(program, &mut static_state, program_symbol).map_err(|e| format!("Error binding program: {}", e))?;

    Ok(Arc::new(static_state))
}

pub fn get_worker_counts(
    workers: Option<usize>,
) -> usize {
    // if we know one but not the other, use the known value for both
    // if we know neither, use the number of CPUs

    let count = match workers {
        Some(v) => v,
        None => workers.unwrap_or_else(|| num_cpus::get()),
    };

    count
}

// pub fn get_main_func_ref(main_collection_symbol: Symbol,
//                          binder: &Binder
// ) -> Result<PointerLive, String> {
//     binder.get_path(vec![main_collection_symbol, "main".to_string()])
// }
//
// pub fn get_func_output_count(fn_ref: &PointerLive, mmu: Arc<MMU>) -> Result<usize, String> {
//     let main_func: FuncLive = get_func_from_ptr(mmu.clone(), &fn_ref.pointer)?;
//     Ok(main_func.output_vals.len())
// }

pub fn log_async(message: String) {
    let stdout = io::stdout();
    let _ = writeln!(&mut stdout.lock(),
                     "{}", message
    );
}

pub fn log_error(msg: String) {
    let stdout = io::stdout();
    let _ = writeln!(&mut stdout.lock(),
                     "\x1B[31m{}\x1B[0m",
                     msg
    );
}


pub fn log_output(func_val: &FuncValLive, value: &PointerLive) {
    let symbol = match &func_val.symbol {
        Some(s) => s,
        None => &func_val.guid,
    };

    let val = jsonify(value.as_ref());
    log_async(format!("out | {}: {}", symbol, val));
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::thread;
    use crate::api::{load_intermediate, stream_call};
    use crate::binder::Binder;
    use crate::binder::intermediate::collection::Collection;
    use crate::binder::json::jsonify;
    use crate::runtime::mmu::mmu::MMU;
    use crate::runtime::vm::manager::StreamResult;

    #[test]
    fn test_stream_with_types() {
        let collection: Collection = load_intermediate("examples/intermediate/test_compiled.json").unwrap();

        let mmu: Arc<MMU> = Arc::new(MMU::new());
        let mut binder = Binder { mmu: mmu.clone(), symbol_table: HashMap::new() };

        binder.store_collection(collection, "my_collection".to_string()).unwrap();

        let main_ref = binder.get_path(vec!["my_collection".into(), "main".into()]).unwrap();

        let (output_sender, output_receiver) = std::sync::mpsc::channel();

        let mmu2 = mmu.clone();
        thread::spawn(move || {
            let _ = stream_call(
                main_ref,
                vec![],
                mmu2,
                true,
                Some(4),
                output_sender
            );
        });

        let mut output_count = 0;
        let mut expected_output_count: Option<usize> = None;
        let mut outputs: HashMap<String, String> = HashMap::new();

        loop {
            let res = output_receiver.recv().unwrap();
            match res {
                StreamResult::NumOutputs(num) => {
                    expected_output_count = Some(num);
                },
                StreamResult::Output(fn_val, val_ref) => {
                    outputs.insert(fn_val.symbol.unwrap_or(fn_val.guid), jsonify(mmu.clone(), &mmu.get_ref_value(&val_ref).unwrap()));
                    output_count += 1;
                    if let Some(expected) = expected_output_count {
                        if output_count >= expected {
                            break;
                        }
                    }
                },
                StreamResult::Error(e) => {
                    panic!("Error: {}", e);
                }
            }
        }

        assert_eq!(outputs.get("age").unwrap(), "60");
        assert_eq!(outputs.get("val").unwrap(), "World");
        assert_eq!(outputs.get("res").unwrap(), "[20, 40, 60]");
    }
}