use std::collections::{HashMap, HashSet};
use std::sync::{Arc, mpsc, RwLock};
use std::sync::atomic::AtomicBool;
use crate::core::data::functions::val::FuncValId;
use crate::core::data::live::{FuncLive, FuncOpLive, FuncValLive, PointerLive};
use crate::core::data::stored::StoredData;
use crate::core::{ExecResult};
use crate::core::data::functions::op::FuncOpId;
use crate::core::vm::functions::v2::orchestrator;
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;

pub type CallContextId = String;
pub type MetaValueId = String;


/// Represents a message sent to the orchestrator to indicate that a new value has been calculated.
#[derive(Clone)]
pub struct NewValMessage<'a> {
    /// The id of the call context that the operation that calculated the value is part of.
    pub call_context_id: CallContextId,

    /// The function value node that this value is for.
    pub func_val: FuncValLive,

    /// A reference to the newly calculated value.
    pub value: ValueReference<'a>
}

/// Represents a message sent to the executor to indicate that a new operation should be executed.
pub struct NewOpMessage {
    /// The id of the call context that the operation belongs to.
    pub call_context_id: CallContextId,

    /// The function operation node that should be executed.
    pub op: FuncOpLive,
}


/// Represents data that is shared between the orchestrator and executor.
pub struct SharedCallState<'a> {
    /// A two-level lookup map for getting the value for a given pair of CallContextId, FuncValId
    /// Note: multiple call contexts/func values can point to the same value.
    /// This will happen if a value is passed as an input/output between function call contexts.
    val_lookup: Arc<RwLock<HashMap<CallContextId, HashMap<FuncValId, ValueReference<'a>>>>>,

    /// A two-level lookup map for storing the remaining outputs for a given call context.
    /// The values of the child map is the linked func val id of the output in the parent call context.
    output_lookup: Arc<RwLock<HashMap<CallContextId, HashMap<FuncValId, (CallContextId, FuncValId)>>>>,

    /// A two-level map for looking up the call context id of a call operation that is inside a given call context.
    call_lookup: Arc<RwLock<HashMap<CallContextId, HashMap<FuncOpId, CallContextId>>>>,

    /// A set of func values that if calculated, will cause a message to be sent back to the main thread.
    // final_outputs: Arc<RwLock<HashSet<(CallContextId, FuncValId)>>>,

    /// A sender for sending outputs back to the main thread.
    // output_sender: mpsc::Sender<NewValMessage<'a>>,

    /// Callback that is called when one of the output values is calculated.
    // output_callback: Box<dyn Fn(CallContextId, &FuncValLive, ValueReference<'a>)>,

    /// Callback that is called when an error occurs.
    // error_callback: Box<dyn Fn(CallContextId, String)>,

    /// The virtual machine that this shared state is associated with.
    pub vm: Arc<VM>,

    new_op_sender: mpsc::Sender<NewOpMessage>,
    new_val_sender: mpsc::Sender<NewValMessage<'a>>,

    halt_flag: Arc<AtomicBool>,

    // TODO: dependent operation queue. set of dependent operations for each val that have not been executed yet.
    // once the queue is empty, the value can be removed from the val_lookup.
}

impl<'a> SharedCallState<'a> {
    /// Creates a new shared call state.
    pub fn new(
        vm: Arc<VM>,
        new_op_sender: mpsc::Sender<NewOpMessage>,
        new_val_sender: mpsc::Sender<NewValMessage<'a>>,
        // func: ValueReference<'a>,
        // args: Vec<ValueReference<'a>>,
        // output_sender: mpsc::Sender<NewValMessage<'a>>,
        // output_callback: Box<dyn Fn(CallContextId, &FuncValLive, ValueReference<'a>)>,
        // error_callback: Box<dyn Fn(CallContextId, String)>
    ) -> Arc<Self> {
        // // generate a random call context id
        // let main_ccid = uuid::Uuid::new_v4().to_string();
        //
        // /// Get the function's outputs
        // let func_live = get_func_from_ptr(vm, &func.pointer).unwrap();
        // let output_fn_vals = get_func_vals_from_ptrs(vm, &func_live.output_vals).unwrap();
        //
        // let final_outputs: HashSet<(CallContextId, FuncValId)> = output_fn_vals.iter()
        //     .map(|val| (main_ccid.clone(), val.guid.clone()))
        //     .collect();

        let state = Arc::new(SharedCallState {
            val_lookup: Arc::new(RwLock::new(HashMap::new())),
            output_lookup: Arc::new(RwLock::new(HashMap::new())),
            call_lookup: Arc::new(Default::default()),
            new_op_sender,
            new_val_sender,
            halt_flag: Arc::new(AtomicBool::new(false)),
            // output_sender,
            // final_outputs: Arc::new(RwLock::new(final_outputs)),
            // output_callback,
            // error_callback,
            vm,
        });

        // match orchestrator::handle_anonymous_fn_call(&state, &main_ccid, &func_live, args) {
        //     Ok(_) => {},
        //     Err(e) => {
        //         state.handle_error(&main_ccid, format!("Error initializing main call context: {}", e));
        //     }
        // }
        state.clone()
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
    pub fn get_val(&self, call_context_id: &CallContextId, func_val: &FuncValLive) -> Option<ValueReference<'a>> {
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
    pub fn set_val(&self, call_context_id: CallContextId, func_val: FuncValLive, value: ValueReference<'a>) {
        let mut val_lookup = self.val_lookup.write().expect("val_lookup lock is poisoned");
        let call_context_map = val_lookup.get_mut(&call_context_id).unwrap_or_else(|| {
            let map = HashMap::new();
            val_lookup.insert(call_context_id.clone(), map);
            val_lookup.get_mut(&call_context_id).unwrap()
        });
        call_context_map.insert(func_val.guid.clone(), value);
    }

    /// Sends a new operation to be executed by the executor.
    pub fn send_new_op(&self, call_context_id: CallContextId, op: FuncOpLive) {
        let message = NewOpMessage {
            call_context_id: call_context_id.clone(),
            op
        };
        self.new_op_sender.send(message).unwrap();
    }

    /// Sends a new value to be handled by the orchestrator.
    pub fn send_new_val(&self, call_context_id: CallContextId, func_val: FuncValLive, value: ValueReference<'a>) {
        let message = NewValMessage {
            call_context_id: call_context_id.clone(),
            func_val,
            value
        };

        // // check if it is a final output
        // if self.final_outputs.read().unwrap().contains(&(call_context_id.clone(), func_val.guid.clone())) {
        //     self.output_sender.send(message.clone()).unwrap();
        // }

        self.new_val_sender.send(message).unwrap();
    }

    /// Drops the values associated with a given call context.
    /// This is used when execution of a function call is complete.
    pub fn drop_call_context(&self, call_context_id: &CallContextId) {
        let mut val_lookup = self.val_lookup.write().expect("val_lookup lock is poisoned");
        val_lookup.remove(call_context_id);
    }

    pub fn handle_error(&self, call_context_id: &CallContextId, error: String) {
        // raise the halt flag
        self.halt_flag.store(true, std::sync::atomic::Ordering::Relaxed);

        // call the error callback
        // TODO: send an async message instead of calling the callback directly.
        // (self.error_callback)(call_context_id.clone(), format!("Call context {} halted: {}", call_context_id, error));

        eprintln!("Call context {} halted: {}", call_context_id, error);
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
    pub fn get_output_info(&self, call_context_id: &CallContextId, func_val: &FuncValLive) -> Option<(CallContextId, FuncValId)> {
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
    pub fn register_output(&self, call_context_id: &CallContextId, func_val: &FuncValLive, parent_call_context_id: &CallContextId) {
        let mut output_lookup = self.output_lookup.write().expect("output_lookup lock is poisoned");
        let outputs = output_lookup.get_mut(call_context_id).unwrap_or_else(|| {
            output_lookup.insert(call_context_id.clone(), HashMap::new());
            output_lookup.get_mut(call_context_id).unwrap()
        });
        outputs.insert(func_val.guid.clone(), (parent_call_context_id.clone(), func_val.guid.clone()));
    }

    pub fn remove_output(&self, call_context_id: &CallContextId, func_val: &FuncValLive) {
        let mut output_lookup = self.output_lookup.write().expect("output_lookup lock is poisoned");
        let outputs = output_lookup.get_mut(call_context_id).unwrap();
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

    pub fn handle_output(&self, call_context_id: &CallContextId, func_val: &FuncValLive, value: ValueReference) {
        // call the output callback
        // TODO: send an async message instead of calling the callback directly.
        // (self.output_callback)(call_context_id.clone(), func_val, value.clone());
        println!("{}: {:?}", match &func_val.symbol {
            None => "Unnamed",
            Some(symbol) => symbol
        }, value);
    }

    /// registers a new call operation with the new call context id that is created within the given parent call context.
    pub fn register_call(
        &self,
        parent_call_context_id: &CallContextId,
        func_op_id: &FuncOpId,
        new_call_context_id: &CallContextId
    ) {
        let mut call_lookup = self.call_lookup.write().expect("call_lookup lock is poisoned");
        let calls = call_lookup.get_mut(parent_call_context_id).unwrap_or_else(|| {
            call_lookup.insert(parent_call_context_id.clone(), HashMap::new());
            call_lookup.get_mut(parent_call_context_id).unwrap()
        });
        calls.insert(func_op_id.clone(), new_call_context_id.clone());
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
        &'a self,
        ptr: PointerLive
    ) -> ExecResult<ValueReference<'a>> {
        self.vm.value_ref_from_ptr(ptr)
    }
}

pub fn get_func_from_ptr(
    vm: Arc<VM>,
    func_ptr: &PointerLive
) -> ExecResult<FuncLive> {
    match vm.get_ptr_value(func_ptr) {
        Ok(value_stored) => match value_stored {
            StoredData::FuncStored(func) => Ok(func),
            _ => return Err(format!("Expected Func, got: {:?}", value_stored))
        }
        Err(e) => return Err(format!("Error getting Func: {}", e))
    }
}

pub fn get_func_val_from_ptr(
    vm: Arc<VM>,
    func_val_ptr: &PointerLive
) -> ExecResult<FuncValLive> {
    match vm.get_ptr_value(func_val_ptr) {
        Ok(value_stored) => match value_stored {
            StoredData::FuncValStored(func_val) => Ok(func_val),
            _ => return Err(format!("Expected FuncVal, got: {:?}", value_stored))
        }
        Err(e) => return Err(format!("Error getting FuncVal: {}", e))
    }
}

pub fn get_func_vals_from_ptrs(vm: Arc<VM>, ptrs: &Vec<PointerLive>) -> ExecResult<Vec<FuncValLive>> {
    ptrs.iter()
        .map(|ptr| get_func_val_from_ptr(vm.clone(), ptr))
        .collect()
}

pub fn get_func_op_from_ptr(
    vm: Arc<VM>,
    func_op_ptr: &PointerLive
) -> ExecResult<FuncOpLive> {
    match vm.get_ptr_value(func_op_ptr) {
        Ok(value_stored) => match value_stored {
            StoredData::FuncOpStored(func_op) => Ok(func_op),
            _ => return Err(format!("Expected FuncOp, got: {:?}", value_stored))

        }
        Err(e) => return Err(format!("Error getting FuncOp: {}", e))
    }
}
