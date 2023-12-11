use crate::core::data::live::live_data::ListLive;
use crate::core::data::live::{IntLive, LiveData};
use crate::core::{ExecResult, Type};
use crate::core::data::stored::StoredData;
use crate::core::gc::{GCPointer};

impl LiveData for ListLive {
    fn type_tag(&self) -> Type {
        Type::List
    }

    fn as_list(&self) -> Option<ExecResult<ListLive>> {
        Some(Ok(self.clone()))
    }

    fn op_len(&self) -> Option<ExecResult<IntLive>> {
        Some(Ok(self.len() as IntLive))
    }

    fn op_get_item(&self, index: &StoredData) -> Option<ExecResult<StoredData>> {
        let index = match index.as_live().as_int() {
            Some(Ok(index)) => index as usize,
            _ => return Some(Err("Index must be an integer")),
        };

        if index >= self.len() {
            return Some(Err("Index out of bounds"));
        }

        match self.get(index) {
            Some(ptr) => ptr.get().map(Ok),
            None => Some(Err("Index out of bounds")),
        }
    }

    fn op_set_item(&self, index: &StoredData, value: GCPointer<StoredData>) -> Option<ExecResult<StoredData>> {
        // copy the list
        let mut list = self.clone();

        let index = match index.as_live().as_int() {
            Some(Ok(index)) => index as usize,
            _ => return Some(Err("Index must be an integer")),
        };

        // get the pointer at the index
        let ptr: &GCPointer<StoredData> = match list.get(index) {
            Some(ptr) => ptr,
            None => return Some(Err("Index out of bounds")),
        };

        // replace the pointer at the index with the new pointer
        list[index] = value;

        // return the new list
        Some(Ok(StoredData::ListStored(list)))
    }

    fn op_push(&self, value: GCPointer<StoredData>) -> Option<ExecResult<StoredData>> {
        let mut list = self.clone();
        list.push(value);
        Some(Ok(StoredData::ListStored(list)))
    }

    fn op_remove(&self, index: &StoredData) -> Option<ExecResult<StoredData>> {
        let mut list = self.clone();

        let index = match index.as_live().as_int() {
            Some(Ok(index)) => index as usize,
            _ => return Some(Err("Index must be an integer")),
        };

        // get the pointer at the index
        let ptr: &GCPointer<StoredData> = match list.get(index) {
            Some(ptr) => ptr,
            None => return Some(Err("Index out of bounds")),
        };

        // remove the pointer at the index
        list.remove(index);

        // return the new list
        Some(Ok(StoredData::ListStored(list)))
    }

    fn op_add(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        let mut lhs: ListLive = self.clone();

        // If casting returns none, then casting to ListLive is not implemented for rhs
        // Return None to indicate that the operation is not supported
        let cast_result: ExecResult<ListLive> = rhs.as_live().as_list()?;

        return cast_result.map(|rhs| {
            // Iterate over rhs_list and add elements to lhs
            for element in rhs {
                lhs.push(element);
            }
            // Convert ListLive to StoredData and wrap in ExecResult
            Ok(StoredData::ListStored(lhs))
        }).ok();
    }
}