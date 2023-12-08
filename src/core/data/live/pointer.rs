use crate::core::data::live::live_data::PointerLive;
use crate::core::data::live::LiveData;
use crate::core::Type;

impl LiveData for PointerLive {
    fn type_tag(&self) -> Type {
        Type::Pointer
    }
}