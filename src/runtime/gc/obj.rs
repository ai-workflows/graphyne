use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct GCObject<T> {
    pub data: T,
    pub ref_count: usize,
    pub phantom: PhantomData<T>,
}

impl<T> Debug for GCObject<T> where T: Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GCObject")
            .field("data", &self.data)
            .field("ref_count", &self.ref_count)
            .finish()
    }
}