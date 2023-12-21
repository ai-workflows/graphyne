use crate::core::data::stored::StoredData;
use crate::core::ExecResult;
use crate::core::vm::value_ref::ValueReference;
use crate::core::vm::VM;
use crate::core::data::live::{LiveData};

impl VM {

    /// Applies a function to each item in a list, returning a new list of the results.
    pub fn map(&self, func: &ValueReference, list: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        let func = self.get_ref_value(func)
            .map_err(|msg| format!("Failed to get function: {}", msg))?
            .as_live().as_func()
            .ok_or_else(|| "Cannot map a non-function value".to_string())??;

        let list_val = self.get_ref_value(list)
            .map_err(|msg| format!("Failed to get list: {}", msg))?
            .as_live().as_list()
            .ok_or_else(|| "Cannot map a function with a non-list value as arguments".to_string())??;

        let result_list: Result<Vec<_>, _> = list_val.iter()
            .map(|item_ptr| {
                self.value_ref_from_ptr(item_ptr.clone())
                    .and_then(|item_val_ref| self.handle_call_function(&func, &[item_val_ref]))
                    .map(|result| result[0].clone())
            })
            .collect();

        result_list.and_then(|vals| {
            let pointers = vals.iter().map(|val_ref| val_ref.pointer.clone()).collect();
            self.store_value(StoredData::ListStored(pointers))
        })
    }


    /// Wrapper function to handle lifetime issues with calling reduce.
    pub fn handle_reduce(&self, func: &ValueReference, list: &ValueReference, initial: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        // create a new reference to the initial value to avoid lifetime issues
        let initial = self.value_ref_from_ptr(initial.pointer.clone())?;

        self.reduce(func, list, &initial)
    }

    // applies a combining function to each item in a list, returning a single result.
    pub fn reduce<'a>(&'a self, func: &ValueReference, list: &ValueReference, initial: &ValueReference<'a>) -> ExecResult<Vec<ValueReference<'a>>> {
        let func = self.get_ref_value(func)
            .map_err(|msg| format!("Failed to get function: {}", msg))?
            .as_live().as_func()
            .ok_or_else(|| "Cannot reduce with a non-function value".to_string())??;

        let list_val = self.get_ref_value(list)
            .map_err(|msg| format!("Failed to get list: {}", msg))?
            .as_live().as_list()
            .ok_or_else(|| "Cannot reduce a list with a non-list value as arguments".to_string())??;

        let mut last_result = initial.clone();

        for item_ptr in list_val {
            let item_val_ref = self.value_ref_from_ptr(item_ptr.clone())?;
            let result_val = self.handle_call_function(&func, &[last_result.clone(), item_val_ref])?;
            last_result = result_val[0].clone();
        }

        Ok(vec![last_result])
    }


    // gets the items in a list that match a given condition
    pub fn filter(&self, func: &ValueReference, list: &ValueReference) -> ExecResult<Vec<ValueReference>> {
        let func = self.get_ref_value(func)
            .map_err(|msg| format!("Failed to get function: {}", msg))?
            .as_live().as_func()
            .ok_or_else(|| "Cannot filter with a non-function value".to_string())??;

        let list_val = self.get_ref_value(list)
            .map_err(|msg| format!("Failed to get list: {}", msg))?
            .as_live().as_list()
            .ok_or_else(|| "Cannot filter a list with a non-list value as arguments".to_string())??;

        let mut result_list: Vec<ValueReference> = Vec::new();

        for item_ptr in list_val {
            let item_val_ref = self.value_ref_from_ptr(item_ptr.clone())?;
            let result_val = self.handle_call_function(&func, &[item_val_ref.clone()])?;
            let result_val_ref = result_val[0].clone();
            let result_val = self.get_ref_value(&result_val_ref)?
                .as_live().as_bool()
                .ok_or_else(|| "Cannot filter a list with a non-bool function".to_string())??;

            if result_val {
                result_list.push(item_val_ref);
            }
        }

        let pointers = result_list.iter().map(|val_ref| val_ref.pointer.clone()).collect();
        self.store_value(StoredData::ListStored(pointers))
    }
}