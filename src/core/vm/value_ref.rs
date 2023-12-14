use std::fmt::Debug;
use crate::core::data::stored::StoredData;
use crate::core::gc::GCPointer;
use crate::core::vm::VM;

pub struct ValueReference<'a> {
    pub pointer: GCPointer<StoredData>,
    pub(crate) vm: &'a VM,
    alive: bool,
}

impl<'a> ValueReference<'a> {
    pub fn new(pointer: GCPointer<StoredData>, vm: &'a VM) -> Self {
        Self {
            pointer,
            vm,
            alive: true,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.alive
    }
}

impl<'a> Drop for ValueReference<'a> {
    fn drop(&mut self) {
        if !self.alive {
            panic!("Cannot drop a dead ValueReference")
        }

        self.vm.drop_reference(self);
    }
}

impl Debug for ValueReference<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValueReference")
            .field("pointer", &self.pointer)
            .field("alive", &self.alive)
            .finish()
    }
}

impl<'a> Clone for ValueReference<'a> {
    fn clone(&self) -> Self {
        if !self.alive {
            panic!("Cannot clone a dead ValueReference")
        }

        self.vm.clone_reference(&self).unwrap()
    }
}