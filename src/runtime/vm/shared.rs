use std::collections::{HashMap, HashSet};
use std::sync::{Arc, mpsc, Mutex, RwLock};
use std::sync::atomic::AtomicBool;
use std::io;
use std::io::Write;
use rayon::{ThreadPool};
use crate::runtime::data::functions::val::FuncValId;
use crate::runtime::data::live::{FuncLive, FuncOpLive, FuncValLive, PointerLive};
use crate::runtime::data::stored::StoredData;
use crate::runtime::{ExecResult, Symbol};
use crate::runtime::data::functions::FuncVal;
use crate::runtime::data::functions::op::FuncOpId;
use crate::runtime::mmu::mmu::{MMU, value_ref_from_ptr};
use crate::runtime::mmu::value_ref::ValueReference;

pub type CallContextId = String;
pub type MetaValueId = String;


/// Represents a message that is sent from the executor to the orchestrator.
pub enum ExecutorMessage {
    NewVal(NewValMessage),
    Pending(ValPendingMessage),
}

/// Represents a message sent to the orchestrator to indicate that a new value has been calculated.
#[derive(Clone)]
pub struct NewValMessage {
    /// The id of the call context that the operation that calculated the value is part of.
    pub call_context_id: CallContextId,

    /// The function value node that this value is for.
    pub func_val: FuncValLive,

    /// A reference to the newly calculated value.
    pub value: ValueReference
}


/// Represents a message sent to the orchestrator to indicate that a value is pending calculation.
pub struct ValPendingMessage {
    pub call_context_id: CallContextId,
    pub func_val: FuncValLive

}

/// Represents a message sent to the executor to indicate that a new operation should be executed.
pub struct NewOpMessage {
    /// The id of the call context that the operation belongs to.
    pub call_context_id: CallContextId,

    /// The function operation node that should be executed.
    pub op: FuncOpLive,
}

#[derive(Debug, Clone)]
pub enum CallResult {
    Success,
    Error(String)
}


/// Represents data that is shared between the orchestrator and executor.
pub struct SharedCallState {
    /// A two-level lookup map for getting the value for a given pair of CallContextId, FuncValId
    /// Note: multiple call contexts/func values can point to the same value.
    /// This will happen if a value is passed as an input/output between function call contexts.
    val_lookup: Arc<RwLock<HashMap<CallContextId, HashMap<FuncValId, ValueReference>>>>,

    /// A two-level lookup map for storing the remaining outputs for a given call context.
    /// The values of the child map is the linked func val id of the output in the parent call context.
    output_lookup: Arc<RwLock<HashMap<CallContextId, HashMap<FuncValId, (CallContextId, FuncVal)>>>>,

    /// A two-level map for looking up the call context id of a call operation that is inside a given call context.
    call_lookup: Arc<RwLock<HashMap<CallContextId, HashMap<FuncOpId, CallContextId>>>>,

    /// A set of func values that if calculated, will cause a message to be sent back to the main thread.
    final_outputs: Arc<RwLock<HashSet<(CallContextId, FuncValId)>>>,

    /// A set of op ids that are currently being executed for a given call context.
    pending_ops: Arc<RwLock<HashMap<CallContextId, HashSet<FuncOpId>>>>,

    /// A sender for sending outputs back to the main thread.
    // output_sender: mpsc::Sender<NewValMessage<'a>>,

    /// Callback that is called when one of the output values is calculated.
    // output_callback: Box<dyn Fn(CallContextId, &FuncValLive, ValueReference<'a>)>,

    /// Callback that is called when an error occurs.
    // error_callback: Box<dyn Fn(CallContextId, String)>,

    /// The virtual machine that this shared state is associated with.
    // pub vm: Arc<VM>,
    pub mmu: Arc<MMU>,

    new_op_sender: mpsc::Sender<NewOpMessage>,
    new_val_sender: mpsc::Sender<NewValMessage>,

    halt_flag: Arc<AtomicBool>,
    results_sender: mpsc::Sender<CallResult>,

    pub executor_thread_pool: Arc<ThreadPool>,
    pub orchestrator_thread_pool: Arc<ThreadPool>,

    pub(crate) verbose: bool

    // TODO: dependent operation queue. set of dependent operations for each val that have not been executed yet.
    // once the queue is empty, the value can be removed from the val_lookup.
}

impl SharedCallState {
    /// Creates a new shared call state.
    pub fn new(
        mmu: Arc<MMU>,
        new_op_sender: mpsc::Sender<NewOpMessage>,
        new_val_sender: mpsc::Sender<NewValMessage>,
        results_sender: mpsc::Sender<CallResult>,
        final_outputs: HashSet<(CallContextId, FuncValId)>,
        ex_pool: Arc<ThreadPool>,
        or_pool: Arc<ThreadPool>,
        verbose: bool
    ) -> Arc<Self> {
        let state = Arc::new(SharedCallState {
            val_lookup: Arc::new(RwLock::new(HashMap::new())),
            output_lookup: Arc::new(RwLock::new(HashMap::new())),
            call_lookup: Arc::new(Default::default()),
            new_op_sender,
            new_val_sender,
            halt_flag: Arc::new(AtomicBool::new(false)),
            results_sender,
            final_outputs: Arc::new(RwLock::new(final_outputs)),
            pending_ops: Arc::new(RwLock::new(HashMap::new())),
            mmu,
            executor_thread_pool: ex_pool,
            orchestrator_thread_pool: or_pool,
            verbose
        });

        state
    }

    /// Checks if a value reference is already stored for a given call context and function value.
    pub fn contains_val(&self, call_context_id: &CallContextId, func_val: &FuncValLive) -> bool {
        let val_lookup = self.val_lookup.read().expect("val_lookup lock is poisoned");
        let call_context_map = match val_lookup.get(call_context_id) {
            Some(map) => map,
            None => return false
        };
        call_context_map.contains_key(&func_val.guid)
    }

    /// Gets the value reference associated with a given call context and function value.
    pub fn get_val(&self, call_context_id: &CallContextId, func_val: &FuncValLive) -> Option<ValueReference> {
        let val_lookup = self.val_lookup.read().expect("val_lookup lock is poisoned");
        let call_context_map = match val_lookup.get(call_context_id) {
            Some(map) => map,
            None => return None
        };
        match call_context_map.get(&func_val.guid) {
            Some(val) => Some(val.clone()),
            None => None
        }
    }

    /// Sets the value reference associated with a given call context and function value.
    pub fn set_val(&self, call_context_id: CallContextId, func_val: FuncValLive, value: ValueReference) {
        let mut val_lookup = self.val_lookup.write().expect("val_lookup lock is poisoned");
        // let call_context_map = val_lookup.get_mut(&call_context_id).unwrap_or_else(|| {
        //     let map = HashMap::new();
        //     val_lookup.insert(call_context_id.clone(), map);
        //     val_lookup.get_mut(&call_context_id).unwrap()
        // });

        if !val_lookup.contains_key(&call_context_id) {
            val_lookup.insert(call_context_id.clone(), HashMap::new());
        }

        let call_context_map = val_lookup.get_mut(&call_context_id).unwrap();
        call_context_map.insert(func_val.guid.clone(), value);
    }

    /// Sends a new operation to be executed by the executor.
    pub fn send_new_op(&self, call_context_id: CallContextId, op: FuncOpLive) {
        // try to register the pending op, do not dispatch if it is already pending
        if !self.try_register_pending_op(&call_context_id, &op.guid) {
            return;
        }

        let message = NewOpMessage {
            call_context_id: call_context_id.clone(),
            op
        };

        let op_code = message.op.opcode.clone();

        // println!("Sending new operation: {:?}", message.op.opcode);
        self.log_async(&call_context_id, &format!("Sending new operation: {:?}", op_code));

        match self.new_op_sender.send(message) {
            Ok(_) => {},
            Err(e) => self.log_error(&call_context_id, &format!("Error sending new operation ({}): {}", op_code, e))
        }
    }

    /// Sends a new value to be handled by the orchestrator.
    pub fn send_new_val(&self, call_context_id: CallContextId, func_val: FuncValLive, value: ValueReference) {
        let message = NewValMessage {
            call_context_id: call_context_id.clone(),
            func_val,
            value
        };

        let symbol: Symbol = match &message.func_val.symbol {
            Some(s) => s.clone(),
            None => message.func_val.guid.clone(),
        };

        self.log_async(&call_context_id, &format!(
            "Sending new value: {} in {}",
            symbol,
            message.call_context_id));

        // // check if it is a final output
        // if self.final_outputs.read().unwrap().contains(&(call_context_id.clone(), func_val.guid.clone())) {
        //     self.output_sender.send(message.clone()).unwrap();
        // }

        match self.new_val_sender.send(message) {
            Ok(_) => {},
            Err(e) => self.log_error(&call_context_id, &format!(
                "Error sending new value ({}): {}", symbol, e))
        }
    }

    /// Drops the values associated with a given call context.
    /// This is used when execution of a function call is complete.
    pub fn drop_call_context(&self, call_context_id: &CallContextId) {
        let mut val_lookup = self.val_lookup.write().expect("val_lookup lock is poisoned");
        val_lookup.remove(call_context_id);
    }

    pub fn halt_execution(&self, call_context_id: &CallContextId, reason: CallResult) {
        // raise the halt flag
        self.halt_flag.store(true, std::sync::atomic::Ordering::Relaxed);

        // println!("Call context {} halted: {}", call_context_id, reason);
        match &reason {
            CallResult::Success => self.log_async(call_context_id, "Call context halted: success"),
            CallResult::Error(msg) => self.log_error(call_context_id, &format!("Call context halted: {}", msg))
        }

        // send the result back to the main thread
        match self.results_sender.send(reason) {
            Ok(_) => {},
            Err(e) => self.log_error(call_context_id, &format!("Error sending result: {}", e))
        }
    }

    pub fn is_halted(&self) -> bool {
        self.halt_flag.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn is_output(&self, call_context_id: &CallContextId, func_val: &FuncValLive) -> bool {
        let output_lookup = self.output_lookup.read().expect("output_lookup lock is poisoned");
        let outputs = match output_lookup.get(call_context_id) {
            Some(outputs) => outputs,
            None => return false
        };
        outputs.contains_key(&func_val.guid)
    }

    /// Gets the parent call context id and id of the func val in the parent call context that matches the given func val.
    pub fn get_output_info(&self, call_context_id: &CallContextId, func_val: &FuncValLive) -> Option<(CallContextId, FuncVal)> {
        let output_lookup = self.output_lookup.read().expect("output_lookup lock is poisoned");
        let outputs = match output_lookup.get(call_context_id) {
            Some(outputs) => outputs,
            None => return None
        };
        match outputs.get(&func_val.guid) {
            Some(info) => Some(info.clone()),
            None => None
        }
    }

    /// Registers the output of a function call.
    pub fn register_output(&self,
                           call_context_id: &CallContextId,
                           func_val: &FuncValLive,
                           parent_call_context_id: &CallContextId,
                           parent_func_val: &FuncValLive

    ) {
        let mut output_lookup = self.output_lookup.write().expect("output_lookup lock is poisoned");
        // let outputs = output_lookup.get_mut(call_context_id).unwrap_or_else(|| {
        //     output_lookup.insert(call_context_id.clone(), HashMap::new());
        //     output_lookup.get_mut(call_context_id).unwrap()
        // });
        // outputs.insert(func_val.guid.clone(), (parent_call_context_id.clone(), func_val.guid.clone()));

        if !output_lookup.contains_key(call_context_id) {
            output_lookup.insert(call_context_id.clone(), HashMap::new());
        }

        if let Some(outputs) = output_lookup.get_mut(call_context_id) {
            outputs.insert(func_val.guid.clone(), (parent_call_context_id.clone(), parent_func_val.clone()));
        }

        self.log_async(&call_context_id, &format!(
            "Registered output: {} linked to {}",
            func_val.symbol.clone().unwrap_or("(unknown symbol)".into()),
            parent_call_context_id));
    }

    pub fn remove_output(&self, call_context_id: &CallContextId, func_val: &FuncValLive) {
        let mut output_lookup = self.output_lookup.write().expect("output_lookup lock is poisoned");
        let outputs = match output_lookup.get_mut(call_context_id) {
            Some(outputs) => outputs,
            None => {
                self.halt_execution(call_context_id, CallResult::Error(format!(
                    "Error removing output ({}): call context {} not found",
                    func_val.symbol.clone().unwrap_or(func_val.guid.clone()),
                    call_context_id)));
                return;
            }
        };

        outputs.remove(&func_val.guid);

        // if the set is empty, remove it from the map
        if outputs.is_empty() {
            output_lookup.remove(call_context_id);
        }
    }

    pub fn num_remaining_outputs(&self, call_context_id: &CallContextId) -> usize {
        let output_lookup = self.output_lookup.read().expect("output_lookup lock is poisoned");
        match output_lookup.get(call_context_id) {
            Some(outputs) => outputs.len(),
            None => 0
        }
    }

    /// registers a new call operation with the new call context id that is created within the given parent call context.
    pub fn register_call(
        &self,
        parent_call_context_id: &CallContextId,
        func_op_id: &FuncOpId,
        new_call_context_id: &CallContextId
    ) {
        let mut call_lookup = self.call_lookup.write().expect("call_lookup lock is poisoned");

        // let mut call_lookup = self.call_lookup.write().expect("call_lookup lock is poisoned");
        // let calls = call_lookup.get_mut(parent_call_context_id).unwrap_or_else(|| {
        //     call_lookup.insert(parent_call_context_id.clone(), HashMap::new());
        //     call_lookup.get_mut(parent_call_context_id).unwrap()
        // });
        // calls.insert(func_op_id.clone(), new_call_context_id.clone());

        if !call_lookup.contains_key(parent_call_context_id) {
            call_lookup.insert(parent_call_context_id.clone(), HashMap::new());
        }

        if let Some(calls) = call_lookup.get_mut(parent_call_context_id) {
            calls.insert(func_op_id.clone(), new_call_context_id.clone());
        }

        self.log_async(&parent_call_context_id, &format!("Registered call -> {}", new_call_context_id));
    }

    pub fn is_call_registered(
        &self,
        parent_call_context_id: &CallContextId,
        func_op_id: &FuncOpId
    ) -> bool {
        let call_lookup = self.call_lookup.read().expect("call_lookup lock is poisoned");
        let calls = match call_lookup.get(parent_call_context_id) {
            Some(calls) => calls,
            None => return false
        };
        calls.contains_key(func_op_id)
    }

    pub fn get_child_call_context_id(
        &self,
        parent_call_context_id: &CallContextId,
        func_op_id: &FuncOpId
    ) -> Option<CallContextId> {
        let call_lookup = self.call_lookup.read().expect("call_lookup lock is poisoned");
        let calls = match call_lookup.get(parent_call_context_id) {
            Some(calls) => calls,
            None => return None
        };
        match calls.get(func_op_id) {
            Some(id) => Some(id.clone()),
            None => None
        }
    }

    pub fn finalize_call_context(&self, call_context_id: &CallContextId) {
        // remove the call context from the call lookup
        let mut call_lookup = self.call_lookup.write().expect("call_lookup lock is poisoned");
        call_lookup.remove(call_context_id);

        // remove the call context from the output lookup
        let mut output_lookup = self.output_lookup.write().expect("output_lookup lock is poisoned");
        output_lookup.remove(call_context_id);

        // remove the call context from the val lookup
        let mut val_lookup = self.val_lookup.write().expect("val_lookup lock is poisoned");
        val_lookup.remove(call_context_id);
    }

    pub fn value_ref_from_ptr(
        &self,
        ptr: PointerLive
    ) -> ExecResult<ValueReference> {
        value_ref_from_ptr(self.mmu.clone(), ptr)
    }

    /// Checks if a new value is a final output. If so, removes it from the final outputs set.
    pub fn check_for_final_output(&self, call_context_id: &CallContextId, func_val: &FuncValLive) -> bool {
        let mut final_outputs = self.final_outputs.write().expect("final_outputs lock is poisoned");
        let is_final_output = final_outputs.contains(&(call_context_id.clone(), func_val.guid.clone()));
        if is_final_output {
            final_outputs.remove(&(call_context_id.clone(), func_val.guid.clone()));
        }
        is_final_output
    }

    /// Checks if there are any remaining final outputs.
    pub fn has_remaining_final_outputs(&self) -> bool {
        let final_outputs = self.final_outputs.read().expect("final_outputs lock is poisoned");
        !final_outputs.is_empty()
    }

    /// Registers an operation as currently pending for a given call context.
    pub fn try_register_pending_op(&self, call_context_id: &CallContextId, op_id: &FuncOpId) -> bool {
        let mut pending_ops = match self.pending_ops.write() {
            Ok(pending_ops) => pending_ops,
            Err(_) => {
                self.halt_execution(call_context_id, CallResult::Error(
                    "Error registering pending op: lock poisoned".to_string()));
                return false;
            }
        };

        if !pending_ops.contains_key(call_context_id) {
            pending_ops.insert(call_context_id.clone(), HashSet::new());
        }

        let call_pending_ops = pending_ops.get_mut(call_context_id).unwrap();

        // return false if the op is already pending
        if call_pending_ops.contains(op_id) {
            return false
        }

        call_pending_ops.insert(op_id.clone());

        true
    }

    /// Checks if an operation is currently pending for a given call context.
    pub fn is_op_pending(&self, call_context_id: &CallContextId, op_id: &FuncOpId) -> bool {
        let pending_ops = self.pending_ops.read().expect("pending_ops lock is poisoned");
        let call_pending_ops = match pending_ops.get(call_context_id) {
            Some(ops) => ops,
            None => return false
        };
        call_pending_ops.contains(op_id)
    }

    /// Marks a pending operation as complete for a given call context.
    pub fn complete_pending_op(&self, call_context_id: &CallContextId, op_id: &FuncOpId) {
        let mut pending_ops = self.pending_ops.write().expect("pending_ops lock is poisoned");
        let call_pending_ops = match pending_ops.get_mut(call_context_id) {
            Some(ops) => ops,
            None => {
                self.halt_execution(call_context_id, CallResult::Error(format!(
                    "Error completing pending op: call context {} not found",
                    call_context_id)));
                return;
            }
        };

        // halt with error if the op is not pending
        if !call_pending_ops.contains(op_id) {
            self.halt_execution(call_context_id, CallResult::Error(format!(
                "Error completing pending op: op {} is not pending",
                op_id)));
        }

        call_pending_ops.remove(op_id);
    }

    pub fn log_async(&self, call_context_id: &CallContextId, msg: &str) {
        if !self.verbose {
            return;
        }

        let stdout = io::stdout();
        let _ = writeln!(&mut stdout.lock(),
                         "[{}] {}",
                         call_context_id,
                         msg
        );
    }

    pub fn log_error(&self, call_context_id: &CallContextId, msg: &str) {
        let stdout = io::stdout();
        let _ = writeln!(&mut stdout.lock(),
                         "\x1B[31m[{}] {}\x1B[0m",
                         call_context_id,
                         msg
        );
    }
}

pub fn get_func_from_ptr(
    mmu: Arc<MMU>,
    func_ptr: &PointerLive
) -> ExecResult<FuncLive> {
    match mmu.get_ptr_value(func_ptr) {
        Ok(value_stored) => match value_stored {
            StoredData::FuncStored(func) => Ok(func),
            _ => return Err(format!("Expected Func, got: {:?}", value_stored))
        }
        Err(e) => return Err(format!("Error getting Func: {}", e))
    }
}

pub fn get_func_val_from_ptr(
    mmu: Arc<MMU>,
    func_val_ptr: &PointerLive
) -> ExecResult<FuncValLive> {
    match mmu.get_ptr_value(func_val_ptr) {
        Ok(value_stored) => match value_stored {
            StoredData::FuncValStored(func_val) => Ok(func_val),
            _ => return Err(format!("Expected FuncVal, got: {:?}", value_stored))
        }
        Err(e) => return Err(format!("Error getting FuncVal: {}", e))
    }
}

pub fn get_func_vals_from_ptrs(mmu: Arc<MMU>, ptrs: &Vec<PointerLive>) -> ExecResult<Vec<FuncValLive>> {
    ptrs.iter()
        .map(|ptr| get_func_val_from_ptr(mmu.clone(), ptr))
        .collect()
}

pub fn get_func_op_from_ptr(
    mmu: Arc<MMU>,
    func_op_ptr: &PointerLive
) -> ExecResult<FuncOpLive> {
    match mmu.get_ptr_value(func_op_ptr) {
        Ok(value_stored) => match value_stored {
            StoredData::FuncOpStored(func_op) => Ok(func_op),
            _ => return Err(format!("Expected FuncOp, got: {:?}", value_stored))

        }
        Err(e) => return Err(format!("Error getting FuncOp: {}", e))
    }
}