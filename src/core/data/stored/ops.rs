use crate::core::data::live::{LiveData};
use crate::core::data::stored::StoredData;
use crate::core::ExecResult;

// TODO: replace err string with runtime errors like cast error, arithmetic error, etc.

/// Add direct operations on StoredData.
impl StoredData {
    pub fn __as_int(&self) -> ExecResult<StoredData> {
        if let Some(result) = self.as_live().as_int() {
            return result.map(|live| StoredData::IntStored(live));
        }

        Err("Cannot cast to int")
    }

    pub fn __as_float(&self) -> ExecResult<StoredData> {
        if let Some(result) = self.as_live().as_float() {
            return result.map(|live| StoredData::FloatStored(live));
        }

        Err("Cannot cast to float")
    }

    pub fn __add(&self, rhs: &StoredData) -> ExecResult<StoredData> {
        if let Some(result) = self.as_live().op_add(rhs) {
            return result;
        }

        Err("Cannot add")
    }

    pub fn __sub(&self, rhs: &StoredData) -> ExecResult<StoredData> {
        if let Some(result) = self.as_live().op_sub(rhs) {
            return result;
        }

        Err("Cannot subtract")
    }

    pub fn __mul(&self, rhs: &StoredData) -> ExecResult<StoredData> {
        if let Some(result) = self.as_live().op_mul(rhs) {
            return result;
        }

        Err("Cannot multiply")
    }

    pub fn __div(&self, rhs: &StoredData) -> ExecResult<StoredData> {
        if let Some(result) = self.as_live().op_div(rhs) {
            return result;
        }

        Err("Cannot divide")
    }
}