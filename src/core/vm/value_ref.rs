use std::fmt::Debug;
use std::sync::Arc;
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::gc::GCPointer;
use crate::core::vm::mmu::mmu::{clone_reference, MMU};

/// ValueReference is a wrapper for a pointer that manages its lifetime.
pub struct ValueReference {
    pub pointer: GCPointer<StoredData>,
    pub mmu: Arc<MMU>,
    alive: bool,
}

impl ValueReference {
    pub fn new(pointer: GCPointer<StoredData>, mmu: Arc<MMU>) -> Self {
        Self {
            pointer,
            mmu,
            alive: true,
        }
    }

    pub fn deref(&self) -> ExecResult<StoredData> {
        self.mmu.get_ptr_value(&self.pointer)
    }

    pub fn is_alive(&self) -> bool {
        self.alive
    }
}

impl Drop for ValueReference {
    fn drop(&mut self) {
        if !self.alive {
            panic!("Cannot drop a dead ValueReference")
        }

        let mmu = self.mmu.clone();

        mmu.drop_reference(self);
    }
}

impl Debug for ValueReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValueReference")
            .field("pointer", &self.pointer)
            .field("alive", &self.alive)
            .finish()
    }
}

impl Clone for ValueReference {
    fn clone(&self) -> Self {
        if !self.alive {
            panic!("Cannot clone a dead ValueReference")
        }

        clone_reference(self.mmu.clone(), &self).unwrap()
    }
}