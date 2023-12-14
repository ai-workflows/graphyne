use std::fmt::Debug;
use std::marker::PhantomData;
use crate::core::gc::{GarbageCollectable};

/// Represents a pointer to a value that is being managed by the garbage collector.
pub struct GCPointer<T> where T: GarbageCollectable<T> {
    pub id: usize,
    pub phantom: PhantomData<T>,

    /// Whether or not this pointer is counted in the ref_count of a GCObject.
    pub counted: bool,
}

impl<T> Debug for GCPointer<T> where T: GarbageCollectable<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GCPointer")
            .field("id", &self.id)
            .field("counted", &self.counted)
            .finish()
    }
}

// impl<T> GCPointer<T> where T: GarbageCollectable<T> {
//
//     pub fn get(&self) -> Option<T> {
//         self.gc.write().unwrap().get(self.id)
//     }
//
//     pub fn ref_count(&self) -> Option<usize> {
//         self.gc.read().unwrap().ref_count(self.id)
//     }
//
//     // pub fn increment_ref(&self) {
//     //     self.gc.write().unwrap().increment_ref(self.id);
//     // }
//     //
//     // pub fn decrement_ref(&self) {
//     //     self.gc.write().unwrap().decrement_ref(self.id);
//     // }
// }

impl<T> PartialEq for GCPointer<T> where T: GarbageCollectable<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}