use std::collections::HashMap;
use crate::core::data::live::live_data::{BoolLive, TypeLive};
use crate::core::data::live::{LiveData, PointerLive};
use crate::core::{ExecResult};
use crate::core::data::live::helpers::type_of_helper;
use crate::core::data::stored::StoredData;
use crate::core::vm::value_ref::ValueReference;

impl LiveData for BoolLive {
    fn type_of(&self, type_map: &HashMap<TypeLive, PointerLive>) -> Option<ExecResult<PointerLive>> {
        type_of_helper(&TypeLive::Boolean, &type_map)
    }

    fn as_bool(&self) -> Option<ExecResult<BoolLive>> {
        Some(Ok(self.clone()))
    }

    fn is_null(&self) -> Option<ExecResult<BoolLive>> {
        Some(Ok(BoolLive::from(false)))
    }

    fn op_if(&self, then: &StoredData, otherwise: &StoredData) -> Option<ExecResult<StoredData>> {
        if *self {
            Some(Ok(then.clone()))
        } else {
            Some(Ok(otherwise.clone()))
        }
    }

    fn op_not(&self) -> Option<ExecResult<StoredData>> {
        Some(Ok(StoredData::BoolStored(!*self)))
    }

    fn op_and(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::BoolStored(rhs) => Some(Ok(StoredData::BoolStored(*self && *rhs))),
            _ => {
                let cast_result: Option<ExecResult<BoolLive>> = rhs.as_live().as_bool();

                cast_result.map(|rhs| {
                    Ok(StoredData::BoolStored(*self && rhs?))
                })
            }
        }
    }

    fn op_or(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::BoolStored(rhs) => Some(Ok(StoredData::BoolStored(*self || *rhs))),
            _ => {
                let cast_result: Option<ExecResult<BoolLive>> = rhs.as_live().as_bool();

                cast_result.map(|rhs| {
                    Ok(StoredData::BoolStored(*self || rhs?))
                })
            }
        }
    }

    fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::BoolStored(rhs) => Some(Ok(StoredData::BoolStored(*self == *rhs))),
            StoredData::NullStored => self.is_null().map(|r| Ok(StoredData::BoolStored(r?))),
            _ => {
                let cast_result: Option<ExecResult<BoolLive>> = rhs.as_live().as_bool();

                cast_result.map(|rhs| {
                    Ok(StoredData::BoolStored(*self == rhs?))
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::data::live::live_data::LiveData;
    use crate::core::vm::ops::Operation;
    use crate::core::vm::mmu::store_op::StoreOp;
    use crate::core::vm::value_ref::ValueReference;
    use crate::core::vm::VM;


    #[test]
    fn test_bool() {
        let mut vm = VM::new(2, 2);

        {
            vm.reset();

            let st_true_result = vm.execute_store(StoreOp::StoreBool(true)).unwrap();
            let st_false_result = vm.execute_store(StoreOp::StoreBool(false)).unwrap();

            let true_ref = st_true_result.get(0).unwrap();
            let false_ref = st_false_result.get(0).unwrap();

            // test bool not
            let not_result = vm.execute_op(Operation::Not(true_ref)).unwrap();
            let not_result = not_result.get(0).unwrap();
            let not_result = vm.get_ref_value(not_result).unwrap().as_live().as_bool().unwrap().ok().unwrap();

            assert_eq!(not_result, false);

            // test bool and
            let and_op = Operation::And(true_ref, false_ref);
            let and_result: Vec<ValueReference> = vm.execute_op(and_op).unwrap();
            let and_result = and_result.get(0).unwrap();
            let and_result = vm.get_ref_value(and_result).unwrap().as_live().as_bool().unwrap().ok().unwrap();

            assert_eq!(and_result, false);

            // test bool or
            let or_op = Operation::Or(true_ref, false_ref);
            let or_result = vm.execute_op(or_op).unwrap();
            let or_result = or_result.get(0).unwrap();
            let or_result = vm.get_ref_value(or_result).unwrap().as_live().as_bool().unwrap().ok().unwrap();

            assert_eq!(or_result, true);

            // test bool eq
            let eq_op = Operation::Equal(true_ref, false_ref);
            let eq_result = vm.execute_op(eq_op).unwrap();
            let eq_result = eq_result.get(0).unwrap();
            let eq_result = vm.get_ref_value(eq_result).unwrap().as_live().as_bool().unwrap().ok().unwrap();

            assert_eq!(eq_result, false);

            // test greater than
            let st_five_result = vm.execute_store(StoreOp::StoreInt(5)).unwrap();
            let st_ten_result = vm.execute_store(StoreOp::StoreInt(10)).unwrap();
            let five_ref = st_five_result.get(0).unwrap();
            let ten_ref = st_ten_result.get(0).unwrap();

            let gt_op = Operation::GreaterThan(ten_ref, five_ref);
            let gt_result = vm.execute_op(gt_op).unwrap();
            let gt_result = gt_result.get(0).unwrap();
            let gt_result = vm.get_ref_value(gt_result).unwrap().as_live().as_bool().unwrap().ok().unwrap();

            assert_eq!(gt_result, true);

            // test less than
            let lt_op = Operation::LessThan(ten_ref, five_ref);
            let lt_result = vm.execute_op(lt_op).unwrap();
            let lt_result = lt_result.get(0).unwrap();
            let lt_result = vm.get_ref_value(lt_result).unwrap().as_live().as_bool().unwrap().ok().unwrap();

            assert_eq!(lt_result, false);

            // test if
            let if_op = Operation::If(true_ref, ten_ref, five_ref);
            let if_result = vm.execute_op(if_op).unwrap();
            let if_result = if_result.get(0).unwrap();
            let if_result = vm.get_ref_value(if_result).unwrap().as_live().as_int().unwrap().ok().unwrap();

            assert_eq!(if_result, 10);

            let if_op = Operation::If(false_ref, ten_ref, five_ref);
            let if_result = vm.execute_op(if_op).unwrap();
            let if_result = if_result.get(0).unwrap();
            let if_result = vm.get_ref_value(if_result).unwrap().as_live().as_int().unwrap().ok().unwrap();

            assert_eq!(if_result, 5);
        }

        // there should be 0 objects in the VM
        assert_eq!(vm.object_count(), 0);

    }
}