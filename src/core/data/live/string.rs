use std::collections::HashMap;
use crate::core::data::live::{BoolLive, FloatLive, IntLive, LiveData, PointerLive, StringLive};
use crate::core::{ExecResult, Type};
use crate::core::data::live::helpers::type_of_helper;
use crate::core::data::live::live_data::TypeLive;
use crate::core::data::stored::StoredData;
use crate::core::vm::value_ref::ValueReference;

impl LiveData for StringLive {
    fn type_of(&self, type_map: &HashMap<TypeLive, PointerLive>) -> Option<ExecResult<PointerLive>> {
        type_of_helper(&TypeLive::String, &type_map)
    }
    fn as_int(&self) -> Option<ExecResult<IntLive>> {
        Some(self.parse::<IntLive>().map_err(|_| "Error parsing int from string".to_string()))
    }

    fn as_float(&self) -> Option<ExecResult<FloatLive>> {
        Some(self.parse::<FloatLive>().map_err(|_| "Error parsing float from string".to_string()))
    }

    fn as_string(&self) -> Option<ExecResult<StringLive>> {
        Some(Ok(self.clone()))
    }

    fn is_null(&self) -> Option<ExecResult<BoolLive>> {
        Some(Ok(BoolLive::from(false)))
    }

    fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        match rhs {
            StoredData::StringStored(rhs) => Some(Ok(StoredData::BoolStored(*self == *rhs))),
            StoredData::NullStored => self.is_null().map(|r| Ok(StoredData::BoolStored(r?))),
            _ => {
                let cast_result: Option<ExecResult<StringLive>> = rhs.as_live().as_string();

                cast_result.map(|rhs| Ok(StoredData::BoolStored(*self == rhs?)))
            }
        }
    }

    fn op_add(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
        // concat
        match rhs {
            StoredData::StringStored(rhs) => {
                Some(Ok(StoredData::StringStored(self.clone() + rhs)))
            }
            _ => {
                let cast_result: Option<ExecResult<StringLive>> = rhs.as_live().as_string();

                cast_result.map(|rhs| {
                    Ok(StoredData::StringStored(self.clone() + &rhs?))
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::data::live::live_data::LiveData;
    use crate::core::vm::ops::Operation;
    use crate::core::vm::store::store_op::StoreOp;
    use crate::core::vm::value_ref::ValueReference;
    use crate::core::vm::VM;

    #[test]
    fn test_concat() {
        let mut vm = VM::new(4);

        test_concat_helper(&mut vm, "test", "test");
        assert_eq!(vm.object_count(), 0);
        test_concat_helper(&mut vm, "test", "test2");
        assert_eq!(vm.object_count(), 0);
        test_concat_helper(&mut vm, "", "test");
        assert_eq!(vm.object_count(), 0);
        test_concat_helper(&mut vm, "test", "");
        assert_eq!(vm.object_count(), 0);
        test_concat_helper(&mut vm, "", "");
        assert_eq!(vm.object_count(), 0);
    }

    fn test_concat_helper(vm: &mut VM, str1: &str, str2: &str) {
        let st_results1: Vec<ValueReference> = vm.execute_store(StoreOp::StoreString(str1.to_string())).unwrap();
        let st_results2: Vec<ValueReference> = vm.execute_store(StoreOp::StoreString(str2.to_string())).unwrap();

        let ptr1 = st_results1.get(0).unwrap();
        let ptr2 = st_results2.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);
        assert_eq!(vm.ref_count(ptr2).unwrap(), 1);

        let concat_op = Operation::Add(ptr1, ptr2);

        let concatenated_results: Vec<ValueReference> = vm.execute_op(concat_op).unwrap();
        let concatenated_ref = concatenated_results.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);

        let concatenated_value = vm.get_ref_value(concatenated_ref).unwrap();

        // println!("{:?}", concatenated_value);

        assert_eq!(concatenated_value.as_live().as_string().unwrap(), Ok(format!("{}{}", str1, str2)));

        // There should be 3 objects in the VM: the two strings and the concatenated string
        assert_eq!(vm.object_count(), 3);
    }

    #[test]
    fn test_eq() {
        let mut vm = VM::new(4);

        test_eq_helper(&mut vm, "test", "test", true);
        assert_eq!(vm.object_count(), 0);
        test_eq_helper(&mut vm, "test", "test2", false);
        assert_eq!(vm.object_count(), 0);
        test_eq_helper(&mut vm, "", "test", false);
        assert_eq!(vm.object_count(), 0);
        test_eq_helper(&mut vm, "test", "", false);
        assert_eq!(vm.object_count(), 0);
        test_eq_helper(&mut vm, "", "", true);
        assert_eq!(vm.object_count(), 0);
    }

    fn test_eq_helper(vm: &mut VM, str1: &str, str2: &str, expected: bool) {
        let st_results1: Vec<ValueReference> = vm.execute_store(StoreOp::StoreString(str1.to_string())).unwrap();
        let st_results2: Vec<ValueReference> = vm.execute_store(StoreOp::StoreString(str2.to_string())).unwrap();

        let ptr1 = st_results1.get(0).unwrap();
        let ptr2 = st_results2.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);
        assert_eq!(vm.ref_count(ptr2).unwrap(), 1);

        let eq_op = Operation::Equal(ptr1, ptr2);

        let eq_results: Vec<ValueReference> = vm.execute_op(eq_op).unwrap();
        let eq_ref = eq_results.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);

        let eq_value = vm.get_ref_value(eq_ref).unwrap();

        // println!("{:?}", eq_value);

        assert_eq!(eq_value.as_live().as_bool().unwrap(), Ok(expected));

        // There should be 3 objects in the VM: the two strings and the concatenated string
        assert_eq!(vm.object_count(), 3);
    }

    #[test]
    fn test_parse_int() {
        let mut vm = VM::new(4);

        test_parse_int_helper(&mut vm, "123", 123);
        assert_eq!(vm.object_count(), 0);
        test_parse_int_helper(&mut vm, "-123", -123);
        assert_eq!(vm.object_count(), 0);
        test_parse_int_helper(&mut vm, "0", 0);
        assert_eq!(vm.object_count(), 0);
        test_parse_int_helper(&mut vm, "1", 1);
        assert_eq!(vm.object_count(), 0);
        test_parse_int_helper(&mut vm, "-1", -1);
        assert_eq!(vm.object_count(), 0);
    }

    fn test_parse_int_helper(vm: &mut VM, str1: &str, expected: i64) {
        let st_results1: Vec<ValueReference> = vm.execute_store(StoreOp::StoreString(str1.to_string())).unwrap();

        let ptr1 = st_results1.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);

        let parse_int_op = Operation::AsInt(ptr1);

        let parse_int_results: Vec<ValueReference> = vm.execute_op(parse_int_op).unwrap();
        let parse_int_ref = parse_int_results.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);

        let parse_int_value = vm.get_ref_value(parse_int_ref).unwrap();

        // println!("{:?}", parse_int_value);

        assert_eq!(parse_int_value.as_live().as_int().unwrap(), Ok(expected));

        // There should be 3 objects in the VM: the two strings and the concatenated string
        assert_eq!(vm.object_count(), 2);
    }

    #[test]
    fn test_parse_float() {
        let mut vm = VM::new(4);

        test_parse_float_helper(&mut vm, "123.0", 123.0);
        assert_eq!(vm.object_count(), 0);
        test_parse_float_helper(&mut vm, "-123.0", -123.0);
        assert_eq!(vm.object_count(), 0);
        test_parse_float_helper(&mut vm, "0.0", 0.0);
        assert_eq!(vm.object_count(), 0);
        test_parse_float_helper(&mut vm, "1.0", 1.0);
        assert_eq!(vm.object_count(), 0);
        test_parse_float_helper(&mut vm, "-1.0", -1.0);
        assert_eq!(vm.object_count(), 0);
        test_parse_float_helper(&mut vm, "1.5", 1.5);
        assert_eq!(vm.object_count(), 0);
        test_parse_float_helper(&mut vm, "-1.5", -1.5);
        assert_eq!(vm.object_count(), 0);
    }

    fn test_parse_float_helper(vm: &mut VM, str1: &str, expected: f64) {
        let st_results1: Vec<ValueReference> = vm.execute_store(StoreOp::StoreString(str1.to_string())).unwrap();

        let ptr1 = st_results1.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);

        let parse_float_op = Operation::AsFloat(ptr1);

        let parse_float_results: Vec<ValueReference> = vm.execute_op(parse_float_op).unwrap();
        let parse_float_ref = parse_float_results.get(0).unwrap();

        assert_eq!(vm.ref_count(ptr1).unwrap(), 1);

        let parse_float_value = vm.get_ref_value(parse_float_ref).unwrap();

        // println!("{:?}", parse_float_value);

        assert_eq!(parse_float_value.as_live().as_float().unwrap(), Ok(expected));

        // There should be 3 objects in the VM: the two strings and the concatenated string
        assert_eq!(vm.object_count(), 2);
    }
}