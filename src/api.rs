use std::collections::HashMap;
use std::{fs, io};
use std::io::Write;
use std::sync::{Arc, mpsc};
use std::sync::mpsc::Sender;
use crate::binder::Binder;
use crate::binder::intermediate::collection::Collection;
use crate::binder::json::jsonify;
use crate::runtime::data::live::{FuncLive, FuncValLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::mmu::mmu::MMU;
use crate::runtime::mmu::value_ref::ValueReference;
use crate::runtime::{ExecResult, Symbol};
use crate::runtime::vm::manager::manage_start_call;
use crate::runtime::vm::shared::{CallResult, get_func_from_ptr, get_func_vals_from_ptrs};

pub fn await_call(
    func: ValueReference,
    args: Vec<ValueReference>,
    mmu: Arc<MMU>,
    verbose: bool,
    execution_workers: Option<usize>,
    orchestration_workers: Option<usize>,
) -> ExecResult<HashMap<Symbol, StoredData>> {
    let (ex_count, or_count) = get_worker_counts(execution_workers, orchestration_workers);
    let ex_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(ex_count).build().unwrap());
    let or_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(or_count).build().unwrap());

    let res = crate::runtime::vm::manager::manage_await_call(
        mmu.clone(),
        ex_pool,
        or_pool,
        func.clone(),
        args,
        verbose
    );

    let res: Vec<ValueReference> = match res {
        Ok(v) => v,
        Err(e) => return Err(format!("Error executing program: {}", e))
    };

    let main_func: FuncLive = match get_func_from_ptr(mmu.clone(), &func.pointer){
        Ok(v) => v,
        Err(e) => return Err(format!("Error getting main function: {}", e))
    };

    let output_fn_vals = match get_func_vals_from_ptrs(mmu.clone(), &main_func.output_vals) {
        Ok(v) => v,
        Err(e) => return Err(format!("Error getting output function values: {}", e))
    };

    let mut result = HashMap::new();

    for (i, output_fn_val) in output_fn_vals.iter().enumerate() {
        let output_val = match res.get(i) {
            Some(v) => v,
            None => return Err(format!("Error getting output value: index {} out of range", i))
        };

        let output_stored = match mmu.get_ref_value(output_val) {
            Ok(v) => v,
            Err(e) => return Err(format!("Error getting output value: {}", e))
        };

        let symbol = match &output_fn_val.symbol {
            Some(s) => s,
            None => &output_fn_val.guid,
        };

        result.insert(symbol.clone(), output_stored);
    }

    Ok(result)
}

pub fn stream_call(
    func: ValueReference,
    args: Vec<ValueReference>,
    mmu: Arc<MMU>,
    outputs_sender: Sender<(Symbol, StoredData)>,
    verbose: bool,
    execution_workers: Option<usize>,
    orchestration_workers: Option<usize>,
) -> ExecResult<()> {
    let (ex_count, or_count) = get_worker_counts(execution_workers, orchestration_workers);
    let ex_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(ex_count).build().unwrap());
    let or_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(or_count).build().unwrap());

    let (result_sender, result_receiver) = mpsc::channel();
    let rs = result_sender.clone();
    let mmu2 = mmu.clone();

    let output_callback = Arc::new(move |message: &crate::runtime::vm::shared::NewValMessage| {

        let stored = match mmu2.get_ref_value(&message.value) {
            Ok(v) => v,
            Err(e) => {
                match result_sender.send(CallResult::Error(format!("Error getting output value: {}", e))) {
                    Ok(_) => {},
                    Err(e2) => log_error(format!("Error sending error ({}) to result receiver: {}", e, e2)),
                }
                return;
            },
        };
        let symbol = match &message.func_val.symbol {
            Some(s) => s,
            None => &message.func_val.guid,
        };

        match outputs_sender.send((symbol.clone(), stored)) {
            Ok(_) => {},
            Err(e) => log_error(format!("Error sending output value: {}", e)),
        }
    });

    let results_channel = (rs, result_receiver);

    let results_receiver = manage_start_call(
        mmu.clone(),
        ex_pool,
        or_pool,
        func,
        args,
        output_callback,
        verbose,
        Some(results_channel)
    );

    match results_receiver.recv().unwrap() {
        CallResult::Success => Ok(()),
        CallResult::Error(e) => Err(e),
    }
}

/// Loads a Graphite JSON Intermediate Language (GJIL) file from the given path and binds to memory.
pub fn load_intermediate(path: &str) -> Result<Collection, String> {
    let contents = fs::read_to_string(path).map_err(|e| format!("Error reading intermediate file: {}", e))?;
    let program: Collection = serde_json::from_str(&contents).map_err(|e| format!("Error parsing intermediate JSON: {}", e))?;

    Ok(program)
}

pub fn bind(program: Collection, mmu: Arc<MMU>, program_symbol: Option<Symbol>) -> Result<Binder, String> {
    let program_symbol = program_symbol.unwrap_or_else(|| "main".to_string());
    let mut binder = Binder { mmu, symbol_table: HashMap::new() };
    binder.store_collection(program, program_symbol).map_err(|e| format!("Error binding program: {}", e))?;

    Ok(binder)
}

pub fn get_worker_counts(
    execution_workers: Option<usize>,
    orchestration_workers: Option<usize>,
) -> (usize, usize) {
    // if we know one but not the other, use the known value for both
    // if we know neither, use the number of CPUs

    let ex_count = match execution_workers {
        Some(v) => v,
        None => orchestration_workers.unwrap_or_else(|| num_cpus::get()),
    };

    let or_count = match orchestration_workers {
        Some(v) => v,
        None => execution_workers.unwrap_or_else(|| num_cpus::get()),
    };

    (ex_count, or_count)
}

pub fn get_main_func_ref(main_collection_symbol: Symbol,
                         binder: &Binder
) -> Result<ValueReference, String> {
    binder.get_path(vec![main_collection_symbol, "main".to_string()])
}

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


pub fn log_output(mmu: Arc<MMU>, func_val: &FuncValLive, value: &ValueReference) {
    let symbol = match &func_val.symbol {
        Some(s) => s,
        None => &func_val.guid,
    };

    let stored = match mmu.get_ref_value(value) {
        Ok(v) => v,
        Err(e) => {
            log_error(format!("Error getting output value: {}", e));
            return;
        },
    };
    let val = jsonify(mmu.clone(), &stored);
    log_async(format!("out | {}: {}", symbol, val));
}