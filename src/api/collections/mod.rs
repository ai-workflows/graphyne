pub(crate) mod collection;
pub(crate) mod func;
pub(crate) mod c_const;


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use maplit::hashmap;
    use crate::api::collections::c_const::CCData;
    use crate::api::collections::collection::Collection;
    use crate::api::collections::func::{CFnValueNode, CollectionFunc, CollectionFuncGraph};
    use crate::api::functions::FunctionOpNode;
    use crate::api::GraphiteApi;
    use crate::api::interface::VmInterface;
    use crate::core::data::live::LiveData;
    use crate::core::data::functions::OpCode;
    use crate::core::vm::VM;

    #[test]
    fn test_collection() {
        let vm: &mut VM = &mut VM::new(4);

        {
            let mut api = GraphiteApi { vm, symbol_table: HashMap::new() };

            let my_list = vec![10, 20, 30];

            let collection = Collection {
                constants: hashmap! {
            "two".into() => 2.into(),
            "my_list".into() => my_list.iter().map(|v| v.clone().into()).collect::<Vec<CCData>>().into(),
            "my_dict".into() => hashmap!{
                "Hello".to_string() => "World".to_string().into(),
                "Foo".to_string() => "Bar".to_string().into()
            }.into(),
        },
                functions: hashmap! {
        "double".into() => CollectionFunc {graph: CollectionFuncGraph {
            values: vec![
                CFnValueNode::constant("_two".into(), CCData::String("two".to_string())),
                CFnValueNode::var("two".into()),

                CFnValueNode::var("num".into()),
                CFnValueNode::var("doubled".into()),
            ],
            ops: vec![
                // get the 2 const
                FunctionOpNode::new(OpCode::Get, vec!["self".into(), "_two".into()], "two".into()),

                // double the number
                FunctionOpNode::new(OpCode::Mul, vec!["num".into(), "two".into()], "doubled".into())
            ],
            input_vals: vec!["num".into()],
            output_vals: vec!["doubled".into()],
        }},
        "double_list".into() => CollectionFunc {graph: CollectionFuncGraph {
            values: vec![
                CFnValueNode::var("double_func".into()),
                CFnValueNode::constant("_double".into(), CCData::String("double".to_string())),
                CFnValueNode::constant("_my_list".into(), CCData::String("my_list".to_string())),

                CFnValueNode::var("my_list".into()),
                CFnValueNode::var("double_list".into()),
            ],
            ops: vec![
                // get the double func and the list
                FunctionOpNode::new(OpCode::Get, vec!["self".into(), "_double".into()], "double_func".into()),
                FunctionOpNode::new(OpCode::Get, vec!["self".into(), "_my_list".into()], "my_list".into()),

                FunctionOpNode::new(OpCode::Map, vec!["double_func".into(), "my_list".into()], "double_list".into())
            ],
            input_vals: vec![],
            output_vals: vec!["double_list".into()],
        }},
        },
                collections: hashmap! {},
                imports: hashmap! {},
            };

            api.store_collection(collection, "my_collection".to_string()).unwrap();

            api.execute(vec!["my_collection".to_string(), "double_list".to_string()], vec![], vec!["doubled_list".to_string()]).unwrap();

            let result = api.get("doubled_list".to_string()).unwrap();
            let result = result.as_live().as_list().unwrap().unwrap();

            assert_eq!(result.len(), 3);

            for i in 0..result.len() {
                let item = result.get(i).unwrap();
                let item_ref = vm.value_ref_from_ptr(item.clone()).unwrap();
                let item = vm.get_ref_value(&item_ref).unwrap();
                let item = item.as_live().as_int().unwrap().unwrap();

                assert_eq!(item, my_list[i] * 2);
            }
        }

        // there should be 0 objects in the VM
        assert_eq!(vm.object_count(), 0);

    }

    #[test]
    fn test_collection_serialization() {
        let vm: &mut VM = &mut VM::new(4);

        {
            let mut api = GraphiteApi { vm, symbol_table: HashMap::new() };

            let json_collection = r#"{
                "constants": {
                    "two": 2,
                    "my_list": [10, 20, 30],
                    "my_dict": {
                        "Hello": "World",
                        "Foo": "Bar"
                    }
                },
                "functions": {
                    "double": {
                       "name": "Double",
                       "description": "Doubles a number",
                       "graph": {
                            "values": [
                                ["_two", "two"],
                                "two",
                                "num",
                                "doubled"
                            ],
                            "ops": [
                                ["Get", ["self", "_two"], "two"],
                                ["Mul", ["num", "two"], "doubled"]
                            ],
                            "input_vals": ["num"],
                            "output_vals": ["doubled"]
                        }
                    },
                    "double_list": {
                        "name": "Double List",
                        "description": "Doubles a list of numbers",
                        "graph": {
                            "values": [
                                "double_func",
                                ["_double", "double"],
                                ["_my_list", "my_list"],
                                "my_list",
                                "double_list"
                            ],
                            "ops": [
                                ["Get", ["self", "_double"], "double_func"],
                                ["Get", ["self", "_my_list"], "my_list"],
                                ["Map", ["double_func", "my_list"], "double_list"]
                            ],
                            "input_vals": [],
                            "output_vals": ["double_list"]
                        }
                    }
                },
                "collections": {},
                "imports": {}
            }"#;

            let collection: Collection = serde_json::from_str(json_collection).unwrap();

            api.store_collection(collection, "my_collection".to_string()).unwrap();

            api.execute(vec!["my_collection".to_string(), "double_list".to_string()], vec![], vec!["doubled_list".to_string()]).unwrap();

            let result = api.get("doubled_list".to_string()).unwrap();
            let result = result.as_live().as_list().unwrap().unwrap();

            assert_eq!(result.len(), 3);
            let my_list = vec![10, 20, 30];

            for i in 0..result.len() {
                let item = result.get(i).unwrap();
                let item_ref = vm.value_ref_from_ptr(item.clone()).unwrap();
                let item = vm.get_ref_value(&item_ref).unwrap();
                let item = item.as_live().as_int().unwrap().unwrap();

                assert_eq!(item, my_list[i] * 2);
            }
        }

        // there should be 0 objects in the VM
        assert_eq!(vm.object_count(), 0);
    }

    #[test]
    fn test_literal_list() {
        let vm: &mut VM = &mut VM::new(4);

        {
            let mut api = GraphiteApi { vm, symbol_table: HashMap::new() };

            let json_collection = r#"{
                "constants": {
                    "two": 2
                },
                "functions": {
                    "double": {
                       "name": "Double",
                       "description": "Doubles a number",
                       "graph": {
                            "values": [
                                ["_two", "two"],
                                "two",
                                "num",
                                "doubled"
                            ],
                            "ops": [
                                ["Get", ["self", "_two"], "two"],
                                ["Mul", ["num", "two"], "doubled"]
                            ],
                            "input_vals": ["num"],
                            "output_vals": ["doubled"]
                        }
                    },
                    "double_list": {
                        "name": "Double List",
                        "description": "Doubles a list of numbers",
                        "graph": {
                            "values": [
                                "double_func",
                                ["_double", "double"],
                                ["my_list", [1, 2, 3]],
                                "double_list",
                                ["null", null]
                            ],
                            "ops": [
                                ["Get", ["self", "_double"], "double_func"],
                                ["Map", ["double_func", "my_list"], "double_list"]
                            ],
                            "input_vals": [],
                            "output_vals": ["double_list"]
                        }
                    }
                },
                "collections": {},
                "imports": {}
            }"#;



            let collection: Collection = match serde_json::from_str(json_collection) {
                Ok(collection) => collection,
                Err(e) => {
                    println!("{}", e);
                    panic!();
                }
            };

            api.store_collection(collection, "my_collection".to_string()).unwrap();
            api.execute(vec!["my_collection".to_string(), "double_list".to_string()], vec![], vec!["doubled_list".to_string()]).unwrap();

            let result = api.get("doubled_list".to_string()).unwrap();
            let result = result.as_live().as_list().unwrap().unwrap();

            assert_eq!(result.len(), 3);
            let my_list = vec![1, 2, 3];

            for i in 0..result.len() {
                let item = result.get(i).unwrap();
                let item_ref = vm.value_ref_from_ptr(item.clone()).unwrap();
                let item = vm.get_ref_value(&item_ref).unwrap();
                let item = item.as_live().as_int().unwrap().unwrap();

                assert_eq!(item, my_list[i] * 2);
            }
        }

        // there should be 0 objects in the VM
        assert_eq!(vm.object_count(), 0);
    }
}