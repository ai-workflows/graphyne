use crate::core::data::live::live_data::PointerLive;
use crate::core::data::live::LiveData;
use crate::core::{ExecResult, Type};

impl LiveData for PointerLive {
    fn type_tag(&self) -> Type {
        Type::Pointer
    }

    fn as_pointer(&self) -> Option<ExecResult<PointerLive>> {
        Some(Ok(self.clone()))
    }
}