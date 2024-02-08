use std::collections::HashMap;
use crate::core::data::live::{BoolLive, FloatLive, IntLive, LiveData, PointerLive};
use crate::core::{ExecResult, Type};
use crate::core::data::live::helpers::type_of_helper;
use crate::core::data::live::live_data::TypeLive;
use crate::core::data::stored::StoredData;
use crate::core::vm::value_ref::ValueReference;
macro_rules! checked_arithmetic_int_op {
    ($self:ident, $rhs:ident, $op:tt, $op_name:ident) => {
        match $rhs {
            StoredData::IntStored(rhs) => {
                Some(match $self.$op_name(*rhs) {
                    Some(value) => Ok(StoredData::IntStored(value)),
                    None => Err("Overflow Error".to_string()),
                })
            }
            _ => {
                let cast_result: Option<ExecResult<IntLive>> = $rhs.as_live().as_int();

                cast_result.map(|rhs| match $self.$op_name(rhs?) {
                    Some(value) => Ok(StoredData::IntStored(value)),
                    None => Err("Overflow Error".to_string()),
                })
            }
        }
    };
}

impl LiveData for IntLive {
    fn type_of(&self, type_map: &HashMap<TypeLive, PointerLive>) -> Option<ExecResult<PointerLive>> {
        type_of_helper(&TypeLive::Integer, &type_map)
    }

    fn as_int(&self) -> Option<ExecResult<IntLive>> {
        Some(Ok(self.clone()))
    }

    fn as_float(&self) -> Option<ExecResult<FloatLive>> {
        Some(Ok(self.clone() as FloatLive))
    }

    fn is_null(&self) -> Option<ExecResult<BoolLive>> {
        Some(Ok(BoolLive::from(false)))
    }

    fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::IntStored(rhs) => Some(Ok(StoredData::BoolStored(self.clone() == *rhs))),
            StoredData::NullStored => self.is_null().map(|r| Ok(StoredData::BoolStored(r?))),
            _ => {
                let cast_result: Option<ExecResult<IntLive>> = rhs.as_live().as_int();

                cast_result.map(|rhs| Ok(StoredData::BoolStored(self.clone() == rhs?)))
            }
        }
    }

    fn op_lt(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::IntStored(rhs) => Some(Ok(StoredData::BoolStored(self.clone() < *rhs))),
            _ => {
                let cast_result: Option<ExecResult<IntLive>> = rhs.as_live().as_int();

                cast_result.map(|rhs| Ok(StoredData::BoolStored(self.clone() < rhs?)))
            }
        }
    }

    fn op_gt(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::IntStored(rhs) => Some(Ok(StoredData::BoolStored(self.clone() > *rhs))),
            _ => {
                let cast_result: Option<ExecResult<IntLive>> = rhs.as_live().as_int();

                cast_result.map(|rhs| Ok(StoredData::BoolStored(self.clone() > rhs?)))
            }
        }
    }

    fn op_add(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_int_op!(self, rhs, +, checked_add)
    }

    fn op_sub(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_int_op!(self, rhs, -, checked_sub)
    }

    fn op_mul(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_int_op!(self, rhs, *, checked_mul)
    }

    fn op_div(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_int_op!(self, rhs, /, checked_div)
    }

    fn op_mod(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_int_op!(self, rhs, %, checked_rem)
    }

    fn op_pow(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::IntStored(rhs) => Some(Ok(StoredData::IntStored(self.clone().pow(*rhs as u32)))),
            _ => {
                let cast_result: Option<ExecResult<IntLive>> = rhs.as_live().as_int();

                cast_result.map(|rhs| Ok(StoredData::IntStored(self.clone().pow(rhs? as u32))))
            }
        }
    }
}

macro_rules! checked_arithmetic_float_op {
    ($self:ident, $rhs:ident, $op:tt) => {
        match $rhs {
            StoredData::FloatStored(rhs) => {
                // Using direct arithmetic operators
                let value = $self $op rhs;
                Some(Ok(StoredData::FloatStored(value)))
            }
            _ => {
                let cast_result: Option<ExecResult<FloatLive>> = $rhs.as_live().as_float();

                cast_result.map(|rhs| {
                    let value = $self $op rhs?;
                    Ok(StoredData::FloatStored(value))
                })
            }
        }
    };
}

impl LiveData for FloatLive {
    fn type_of(&self, type_map: &HashMap<TypeLive, PointerLive>) -> Option<ExecResult<PointerLive>> {
        type_of_helper(&TypeLive::Float, &type_map)
    }

    fn as_int(&self) -> Option<ExecResult<IntLive>> {
        Some(Ok(self.clone() as IntLive))
    }

    fn as_float(&self) -> Option<ExecResult<FloatLive>> {
        Some(Ok(self.clone()))
    }

    fn is_null(&self) -> Option<ExecResult<BoolLive>> {
        Some(Ok(BoolLive::from(false)))
    }

    fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::FloatStored(rhs) => Some(Ok(StoredData::BoolStored(self.clone() == *rhs))),
            StoredData::NullStored => self.is_null().map(|r| Ok(StoredData::BoolStored(r?))),
            _ => {
                let cast_result: Option<ExecResult<FloatLive>> = rhs.as_live().as_float();

                cast_result.map(|rhs| {
                    Ok(StoredData::BoolStored(self.clone() == rhs?))
                })
            }
        }
    }

    fn op_lt(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::FloatStored(rhs) => Some(Ok(StoredData::BoolStored(self.clone() < *rhs))),
            _ => {
                let cast_result: Option<ExecResult<FloatLive>> = rhs.as_live().as_float();

                cast_result.map(|rhs| {
                    Ok(StoredData::BoolStored(self.clone() < rhs?))
                })
            }
        }
    }

    fn op_gt(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::FloatStored(rhs) => Some(Ok(StoredData::BoolStored(self.clone() > *rhs))),
            _ => {
                let cast_result: Option<ExecResult<FloatLive>> = rhs.as_live().as_float();

                cast_result.map(|rhs| {
                    Ok(StoredData::BoolStored(self.clone() > rhs?))
                })
            }
        }
    }

    fn op_add(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_float_op!(self, rhs, +)
    }

    fn op_sub(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_float_op!(self, rhs, -)
    }

    fn op_mul(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_float_op!(self, rhs, *)
    }

    fn op_div(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_float_op!(self, rhs, /)
    }

    fn op_mod(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        checked_arithmetic_float_op!(self, rhs, %)
    }

    fn op_pow(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::FloatStored(rhs) => {
                // Using direct arithmetic operators
                let value = self.powf(*rhs);
                Some(Ok(StoredData::FloatStored(value)))
            }
            _ => {
                let cast_result: Option<ExecResult<FloatLive>> = rhs.as_live().as_float();

                cast_result.map(|rhs| {
                    let value = self.powf(rhs?);
                    Ok(StoredData::FloatStored(value))
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::data::live::{FloatLive, LiveData};
    use crate::core::data::stored::StoredData;
    use crate::core::data::stored::StoredData::{FloatStored, IntStored};
    use crate::core::vm::ops::Operation;
    use crate::core::vm::store::store_op::StoreOp;
    use crate::core::vm::value_ref::ValueReference;
    use crate::core::vm::VM;

    /// Gets the proper store operation based on the type of the value.
    fn get_store_op<'a>(val: StoredData) -> StoreOp<'a> {
        match val {
            IntStored(val) => StoreOp::StoreInt(val),
            FloatStored(val) => StoreOp::StoreFloat(val),
            StoredData::StringStored(val) => StoreOp::StoreString(val),
            StoredData::BoolStored(val) => StoreOp::StoreBool(val),
            _ => panic!("Invalid value type"),
        }
    }

    #[test]
    fn test_add() {
        let vm = VM::new(2, 2);

        add_helper(&vm, IntStored(1), IntStored(2), 3.0);
        assert_eq!(vm.object_count(), 0);
        add_helper(&vm, IntStored(1), FloatStored(2.0), 3.0);
        assert_eq!(vm.object_count(), 0);
        add_helper(&vm, FloatStored(1.0), IntStored(2), 3.0);
        assert_eq!(vm.object_count(), 0);
        add_helper(&vm, FloatStored(1.0), FloatStored(2.0), 3.0);
        assert_eq!(vm.object_count(), 0);
        add_helper(&vm, IntStored(1), IntStored(-2), -1.0);
        assert_eq!(vm.object_count(), 0);
        add_helper(&vm, IntStored(1), FloatStored(-2.0), -1.0);
        assert_eq!(vm.object_count(), 0);
        add_helper(&vm, FloatStored(1.0), IntStored(-2), -1.0);
        assert_eq!(vm.object_count(), 0);
        add_helper(&vm, FloatStored(1.0), FloatStored(-2.0), -1.0);
        assert_eq!(vm.object_count(), 0);
        add_helper(&vm, IntStored(1), FloatStored(2.5), 3.0);
        assert_eq!(vm.object_count(), 0);
        add_helper(&vm, FloatStored(1.5), IntStored(2), 3.5);
        assert_eq!(vm.object_count(), 0);
        add_helper(&vm, FloatStored(1.5), FloatStored(2.75), 4.25);
        assert_eq!(vm.object_count(), 0);
    }

    fn add_helper(vm: &VM, lhs: StoredData, rhs: StoredData, expected: FloatLive) {
        let results_1: Vec<ValueReference> = vm.execute_store(get_store_op(lhs)).unwrap();
        let results_2: Vec<ValueReference> = vm.execute_store(get_store_op(rhs)).unwrap();

        let ptr1 = results_1.get(0).unwrap();
        let ptr2 = results_2.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);
        assert_eq!(vm.ref_count(ptr2).unwrap(), 1);

        let add = Operation::Add(ptr1, ptr2);

        let add_result = vm.execute_op(add).unwrap();
        let sum_ref = add_result.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);

        let sum_value = vm.get_ref_value(sum_ref).unwrap();

        assert_eq!(sum_value.as_live().as_float().unwrap(), Ok(expected));

        // println!("{:?}", sum_value);

        // There should be 3 objects in the VM: the two ints and the sum
        assert_eq!(vm.object_count(), 3);
    }

    #[test]
    fn test_sub() {
        let vm = VM::new(2, 2);

        sub_helper(&vm, IntStored(1), IntStored(2), -1.0);
        assert_eq!(vm.object_count(), 0);
        sub_helper(&vm, IntStored(1), FloatStored(2.0), -1.0);
        assert_eq!(vm.object_count(), 0);
        sub_helper(&vm, FloatStored(1.0), IntStored(2), -1.0);
        assert_eq!(vm.object_count(), 0);
        sub_helper(&vm, FloatStored(1.0), FloatStored(2.0), -1.0);
        assert_eq!(vm.object_count(), 0);
        sub_helper(&vm, IntStored(1), IntStored(-2), 3.0);
        assert_eq!(vm.object_count(), 0);
        sub_helper(&vm, IntStored(1), FloatStored(-2.0), 3.0);
        assert_eq!(vm.object_count(), 0);
        sub_helper(&vm, FloatStored(1.0), IntStored(-2), 3.0);
        assert_eq!(vm.object_count(), 0);
        sub_helper(&vm, FloatStored(1.0), FloatStored(-2.0), 3.0);
        assert_eq!(vm.object_count(), 0);
        sub_helper(&vm, IntStored(1), FloatStored(2.5), -1.0);
        assert_eq!(vm.object_count(), 0);
        sub_helper(&vm, FloatStored(1.5), IntStored(2), -0.5);
        assert_eq!(vm.object_count(), 0);
        sub_helper(&vm, FloatStored(1.5), FloatStored(2.75), -1.25);
        assert_eq!(vm.object_count(), 0);
    }

    fn sub_helper(vm: &VM, lhs: StoredData, rhs: StoredData, expected: FloatLive) {
        let results_1: Vec<ValueReference> = vm.execute_store(get_store_op(lhs)).unwrap();
        let results_2: Vec<ValueReference> = vm.execute_store(get_store_op(rhs)).unwrap();

        let ptr1 = results_1.get(0).unwrap();
        let ptr2 = results_2.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);
        assert_eq!(vm.ref_count(ptr2).unwrap(), 1);

        let sub = Operation::Sub(ptr1, ptr2);

        let sub_result = vm.execute_op(sub).unwrap();
        let diff_ref = sub_result.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);

        let diff_value = vm.get_ref_value(diff_ref).unwrap();

        assert_eq!(diff_value.as_live().as_float().unwrap(), Ok(expected));

        // println!("{:?}", diff_value);

        // There should be 3 objects in the VM: the two ints and the diff
        assert_eq!(vm.object_count(), 3);
    }

    #[test]
    fn test_mul() {
        let vm = VM::new(2, 2);

        mul_helper(&vm, IntStored(1), IntStored(2), 2.0);
        assert_eq!(vm.object_count(), 0);
        mul_helper(&vm, IntStored(1), FloatStored(2.0), 2.0);
        assert_eq!(vm.object_count(), 0);
        mul_helper(&vm, FloatStored(1.0), IntStored(2), 2.0);
        assert_eq!(vm.object_count(), 0);
        mul_helper(&vm, FloatStored(1.0), FloatStored(2.0), 2.0);
        assert_eq!(vm.object_count(), 0);
        mul_helper(&vm, IntStored(1), IntStored(-2), -2.0);
        assert_eq!(vm.object_count(), 0);
        mul_helper(&vm, IntStored(1), FloatStored(-2.0), -2.0);
        assert_eq!(vm.object_count(), 0);
        mul_helper(&vm, FloatStored(1.0), IntStored(-2), -2.0);
        assert_eq!(vm.object_count(), 0);
        mul_helper(&vm, FloatStored(1.0), FloatStored(-2.0), -2.0);
        assert_eq!(vm.object_count(), 0);
        mul_helper(&vm, IntStored(1), FloatStored(2.5), 2.0);
        assert_eq!(vm.object_count(), 0);
        mul_helper(&vm, FloatStored(1.5), IntStored(2), 3.0);
        assert_eq!(vm.object_count(), 0);
        mul_helper(&vm, FloatStored(1.5), FloatStored(2.75), 4.125);
        assert_eq!(vm.object_count(), 0);
    }

    fn mul_helper(vm: &VM, lhs: StoredData, rhs: StoredData, expected: FloatLive) {
        let results_1: Vec<ValueReference> = vm.execute_store(get_store_op(lhs)).unwrap();
        let results_2: Vec<ValueReference> = vm.execute_store(get_store_op(rhs)).unwrap();

        let ptr1 = results_1.get(0).unwrap();
        let ptr2 = results_2.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);
        assert_eq!(vm.ref_count(ptr2).unwrap(), 1);

        let mul = Operation::Mul(ptr1, ptr2);

        let mul_result = vm.execute_op(mul).unwrap();
        let prod_ref = mul_result.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);

        let prod_value = vm.get_ref_value(prod_ref).unwrap();

        assert_eq!(prod_value.as_live().as_float().unwrap(), Ok(expected));

        // println!("{:?}", prod_value);

        // There should be 3 objects in the VM: the two ints and the prod
        assert_eq!(vm.object_count(), 3);
    }

    #[test]
    fn test_div() {
        let vm = VM::new(2, 2);

        div_helper(&vm, IntStored(1), IntStored(2), 0.0);
        assert_eq!(vm.object_count(), 0);
        div_helper(&vm, IntStored(1), FloatStored(2.0), 0.0);
        assert_eq!(vm.object_count(), 0);
        div_helper(&vm, FloatStored(1.0), IntStored(2), 0.5);
        assert_eq!(vm.object_count(), 0);
        div_helper(&vm, FloatStored(1.0), FloatStored(2.0), 0.5);
        assert_eq!(vm.object_count(), 0);
        div_helper(&vm, IntStored(1), IntStored(-2), 0.0);
        assert_eq!(vm.object_count(), 0);
        div_helper(&vm, IntStored(1), FloatStored(-2.0), 0.0);
        assert_eq!(vm.object_count(), 0);
        div_helper(&vm, FloatStored(1.0), IntStored(-2), -0.5);
        assert_eq!(vm.object_count(), 0);
        div_helper(&vm, FloatStored(1.0), FloatStored(-2.0), -0.5);
        assert_eq!(vm.object_count(), 0);
        div_helper(&vm, IntStored(1), FloatStored(2.5), 0.0);
        assert_eq!(vm.object_count(), 0);
        div_helper(&vm, FloatStored(1.5), IntStored(2), 0.75);
        assert_eq!(vm.object_count(), 0);
        div_helper(&vm, FloatStored(1.5), FloatStored(2.5), 0.6);
        assert_eq!(vm.object_count(), 0);
    }

    fn div_helper(vm: &VM, lhs: StoredData, rhs: StoredData, expected: FloatLive) {
        let results_1: Vec<ValueReference> = vm.execute_store(get_store_op(lhs)).unwrap();
        let results_2: Vec<ValueReference> = vm.execute_store(get_store_op(rhs)).unwrap();

        let ptr1 = results_1.get(0).unwrap();
        let ptr2 = results_2.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);
        assert_eq!(vm.ref_count(ptr2).unwrap(), 1);

        let div = Operation::Div(ptr1, ptr2);

        let div_result = vm.execute_op(div).unwrap();
        let quot_ref = div_result.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);

        let quot_value = vm.get_ref_value(quot_ref).unwrap();

        assert_eq!(quot_value.as_live().as_float().unwrap(), Ok(expected));

        // println!("{:?}", quot_value);

        // There should be 3 objects in the VM: the two ints and the quot
        assert_eq!(vm.object_count(), 3);
    }

    #[test]
    fn test_mod() {
        let vm = VM::new(2, 2);

        mod_helper(&vm, IntStored(1), IntStored(2), 1.0);
        assert_eq!(vm.object_count(), 0);
        mod_helper(&vm, IntStored(1), FloatStored(2.0), 1.0);
        assert_eq!(vm.object_count(), 0);
        mod_helper(&vm, FloatStored(1.0), IntStored(2), 1.0);
        assert_eq!(vm.object_count(), 0);
        mod_helper(&vm, FloatStored(1.0), FloatStored(2.0), 1.0);
        assert_eq!(vm.object_count(), 0);

        mod_helper(&vm, IntStored(2), IntStored(2), 0.0);
        assert_eq!(vm.object_count(), 0);
        mod_helper(&vm, IntStored(2), FloatStored(2.0), 0.0);
        assert_eq!(vm.object_count(), 0);
        mod_helper(&vm, FloatStored(2.0), IntStored(2), 0.0);
        assert_eq!(vm.object_count(), 0);
        mod_helper(&vm, FloatStored(2.0), FloatStored(2.0), 0.0);
        assert_eq!(vm.object_count(), 0);
    }

    fn mod_helper(vm: &VM, lhs: StoredData, rhs: StoredData, expected: FloatLive) {
        let results_1: Vec<ValueReference> = vm.execute_store(get_store_op(lhs)).unwrap();
        let results_2: Vec<ValueReference> = vm.execute_store(get_store_op(rhs)).unwrap();

        let ptr1 = results_1.get(0).unwrap();
        let ptr2 = results_2.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);
        assert_eq!(vm.ref_count(ptr2).unwrap(), 1);

        let rem = Operation::Mod(ptr1, ptr2);

        let rem_result = vm.execute_op(rem).unwrap();
        let rem_ref = rem_result.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);

        let rem_value = vm.get_ref_value(rem_ref).unwrap();

        assert_eq!(rem_value.as_live().as_float().unwrap(), Ok(expected));

        // println!("{:?}", rem_value);

        // There should be 3 objects in the VM: the two ints and the rem
        assert_eq!(vm.object_count(), 3);
    }

    #[test]
    fn test_pow() {
        let vm = VM::new(2, 2);

        pow_helper(&vm, IntStored(1), IntStored(2), 1.0);
        assert_eq!(vm.object_count(), 0);
        pow_helper(&vm, IntStored(1), FloatStored(2.0), 1.0);
        assert_eq!(vm.object_count(), 0);
        pow_helper(&vm, FloatStored(1.0), IntStored(2), 1.0);
        assert_eq!(vm.object_count(), 0);
        pow_helper(&vm, FloatStored(1.0), FloatStored(2.0), 1.0);
        assert_eq!(vm.object_count(), 0);

        pow_helper(&vm, IntStored(2), IntStored(2), 4.0);
        assert_eq!(vm.object_count(), 0);
        pow_helper(&vm, IntStored(2), FloatStored(2.0), 4.0);
        assert_eq!(vm.object_count(), 0);
        pow_helper(&vm, FloatStored(2.0), IntStored(2), 4.0);
        assert_eq!(vm.object_count(), 0);

        pow_helper(&vm, IntStored(2), IntStored(3), 8.0);
        assert_eq!(vm.object_count(), 0);
        pow_helper(&vm, IntStored(2), FloatStored(3.0), 8.0);
        assert_eq!(vm.object_count(), 0);
        pow_helper(&vm, FloatStored(2.0), IntStored(3), 8.0);
        assert_eq!(vm.object_count(), 0);
    }

    fn pow_helper(vm: &VM, lhs: StoredData, rhs: StoredData, expected: FloatLive) {
        let results_1: Vec<ValueReference> = vm.execute_store(get_store_op(lhs)).unwrap();
        let results_2: Vec<ValueReference> = vm.execute_store(get_store_op(rhs)).unwrap();

        let ptr1 = results_1.get(0).unwrap();
        let ptr2 = results_2.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);
        assert_eq!(vm.ref_count(ptr2).unwrap(), 1);

        let pow = Operation::Pow(ptr1, ptr2);

        let pow_result = vm.execute_op(pow).unwrap();
        let pow_ref = pow_result.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);

        let pow_value = vm.get_ref_value(pow_ref).unwrap();

        assert_eq!(pow_value.as_live().as_float().unwrap(), Ok(expected));

        // println!("{:?}", pow_value);

        // There should be 3 objects in the VM: the two ints and the pow
        assert_eq!(vm.object_count(), 3);
    }
}