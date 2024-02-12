use std::collections::HashMap;
use std::{fs, io};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use crate::binder::Binder;
use crate::binder::intermediate::collection::Collection;
use crate::binder::json::jsonify;
use crate::runtime::data::live::{FuncLive, FuncValLive};
use crate::runtime::mmu::mmu::MMU;
use crate::runtime::mmu::value_ref::ValueReference;
use crate::runtime::{ExecResult, Symbol};
use crate::runtime::vm::manager::{manage_start_call, StreamResult};
use crate::runtime::vm::shared::{get_func_from_ptr, get_func_vals_from_ptrs};

pub fn await_call(
    func: ValueReference,
    args: Vec<ValueReference>,
    mmu: Arc<MMU>,
    verbose: bool,
    execution_workers: Option<usize>,
    orchestration_workers: Option<usize>,
) -> ExecResult<HashMap<Symbol, ValueReference>> {
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

        let symbol = match &output_fn_val.symbol {
            Some(s) => s,
            None => &output_fn_val.guid,
        };

        result.insert(symbol.clone(), output_val.clone());
    }

    Ok(result)
}

pub fn stream_call(
    func: ValueReference,
    args: Vec<ValueReference>,
    mmu: Arc<MMU>,
    outputs_sender: Sender<StreamResult>,
    verbose: bool,
    execution_workers: Option<usize>,
    orchestration_workers: Option<usize>,
) -> ExecResult<usize> {
    let (ex_count, or_count) = get_worker_counts(execution_workers, orchestration_workers);
    let ex_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(ex_count).build().unwrap());
    let or_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(or_count).build().unwrap());

    let outputs_sender = Arc::new(Mutex::new(outputs_sender));

    let num_expected_outputs = manage_start_call(
        mmu.clone(),
        ex_pool,
        or_pool,
        func,
        args,
        outputs_sender,
        verbose,
    );

    match num_expected_outputs {
        Ok(v) => Ok(v),
        Err(e) => Err(format!("Error executing program: {}", e))
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

pub fn get_func_output_count(fn_ref: &ValueReference, mmu: Arc<MMU>) -> Result<usize, String> {
    let main_func: FuncLive = get_func_from_ptr(mmu.clone(), &fn_ref.pointer)?;
    Ok(main_func.output_vals.len())
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


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use crate::api::{load_intermediate, stream_call};
    use crate::binder::Binder;
    use crate::binder::intermediate::collection::Collection;
    use crate::binder::json::jsonify;
    use crate::runtime::mmu::mmu::MMU;
    use crate::runtime::vm::manager::StreamResult;

    #[test]
    fn test_stream_with_types() {
        let collection: Collection = load_intermediate("examples/test/test_compiled.json").unwrap();

        let mmu: Arc<MMU> = Arc::new(MMU::new());
        let mut binder = Binder { mmu: mmu.clone(), symbol_table: HashMap::new() };

        binder.store_collection(collection, "my_collection".to_string()).unwrap();

        let main_ref = binder.get_path(vec!["my_collection".into(), "main".into()]).unwrap();

        let (output_sender, output_receiver) = std::sync::mpsc::channel();

        let num_expected_outputs = stream_call(
            main_ref,
            vec![],
            mmu.clone(),
            output_sender,
            true,
            Some(4),
            Some(4),
        ).unwrap();

        let mut outputs: HashMap<String, String> = HashMap::new();

        for _ in 0..num_expected_outputs {
            let result = output_receiver.recv().unwrap();
            match result {
                StreamResult::Output(fn_val, val_ref) => {
                    let symbol = match &fn_val.symbol {
                        Some(s) => s,
                        None => &fn_val.guid,
                    };
                    let val = jsonify(mmu.clone(), &mmu.get_ref_value(&val_ref).unwrap());
                    outputs.insert(symbol.clone(), val);
                }
                StreamResult::Error(e) => panic!("Error: {}", e),
            }
        }

        assert_eq!(outputs.get("age").unwrap(), "60");
        assert_eq!(outputs.get("val").unwrap(), "World");
        assert_eq!(outputs.get("res").unwrap(), "[20, 40, 60]");
    }
}