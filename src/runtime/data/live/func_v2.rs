use std::sync::Arc;
use crate::runtime::data::functions::v2::FuncV2;
use crate::runtime::data::live::{LiveData, PointerLive, TypeLive};
use crate::runtime::ExecResult;
use crate::runtime::static_state::state::StaticState;

impl LiveData for FuncV2 {
    fn type_of(&self, type_map: Arc<StaticState>) -> Option<ExecResult<PointerLive>> {
        type_map.get_primitive_type(&TypeLive::Function).map(|r| Ok(r))
    }

    // fn as_func(&self) -> Option<ExecResult<FuncLive>> {
    //     Some(Ok(self.clone()))
    // }
    //
    // fn is_null(&self) -> Option<ExecResult<BoolLive>> {
    //     Some(Ok(BoolLive::from(false)))
    // }
    //
    // fn op_eq(&self, rhs: &StoredData) -> Option<ExecResult<StoredData>> {
    //     match rhs {
    //         StoredData::NullStored => self.is_null().map(|r| Ok(StoredData::BoolStored(r?))),
    //         _ => None,
    //     }
    // }
}