use std::{fs, io};
use std::io::Write;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use crate::binder::binder;
use crate::binder::intermediate::collection::Collection;
use crate::runtime::data::live::PointerLive;
use crate::runtime::{Symbol, SymbolPath};
use crate::runtime::static_state::state::StaticState;
use crate::runtime::vm::manager::{init_await_call, init_stream_call};

pub fn await_call(
    main_symbol_path: SymbolPath,
    inputs: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    workers: Option<usize>,
) -> Vec<PointerLive> {
    let worker_count = get_worker_counts(workers);
    let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(worker_count).build().unwrap());

    init_await_call(
        main_symbol_path,
        inputs,
        static_state,
        worker_pool
    )
}

pub fn stream_call(
    main_symbol_path: SymbolPath,
    inputs: Vec<PointerLive>,
    static_state: Arc<StaticState>,
    workers: Option<usize>,
) -> (usize, Receiver<(usize, PointerLive)>) {
    let worker_count = get_worker_counts(workers);
    let worker_pool = Arc::new(rayon::ThreadPoolBuilder::new().num_threads(worker_count).build().unwrap());

    init_stream_call(
        main_symbol_path,
        inputs,
        static_state,
        worker_pool
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

    match binder::bind_program(program, &mut static_state, program_symbol) {
        Ok(_) => {},
        Err(e) => return Err(format!("Error binding program: {}", e)),
    }

    Ok(Arc::new(static_state))
}

pub fn get_worker_counts(workers: Option<usize>) -> usize {
    match workers {
        Some(0) => 1,
        Some(v) => v,
        None => num_cpus::get(),
    }
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use crate::api::{bind, get_worker_counts, load_intermediate, stream_call};
    use crate::binder::intermediate::collection::Collection;
    use crate::binder::json::jsonify;
    use crate::runtime::data::live::PointerLive;

    #[test]
    fn test_stream_with_types() {
        let collection: Collection = load_intermediate("examples/intermediate/test_compiled.json").unwrap();

        let static_state = bind(collection, Some("top_level".to_string())).unwrap();

        let (output_count, output_receiver) = stream_call(
            vec!["top_level".to_string(), "main".to_string()],
            vec![],
            static_state.clone(),
            Some(4)
        );

        let mut outputs: HashMap<usize, PointerLive> = HashMap::new();

        for _ in 0..output_count {
            let (idx, val) = output_receiver.recv().unwrap();
            println!("output {}: {}", idx, jsonify(val.as_ref()));
            outputs.insert(idx, val);
        }

        let res = outputs.get(&0).unwrap().stored_as_list().unwrap();
        let res: Vec<i64> = res.iter().map(|v| *v.stored_as_int().unwrap()).collect();
        assert_eq!(res, vec![20, 40, 60]);

        let val = outputs.get(&1).unwrap().stored_as_string().unwrap();
        assert_eq!(val, "World");

        let age = outputs.get(&2).unwrap().stored_as_int().unwrap();
        assert_eq!(*age, 60);
    }

    #[test]
    fn zero_workers_falls_back_to_one_thread() {
        assert_eq!(get_worker_counts(Some(0)), 1);
    }

    #[test]
    fn none_workers_uses_cpu_count() {
        assert_eq!(get_worker_counts(None), num_cpus::get());
    }
}
