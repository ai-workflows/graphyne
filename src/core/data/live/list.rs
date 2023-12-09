use crate::core::data::live::live_data::ListLive;
use crate::core::data::live::LiveData;
use crate::core::{ExecResult, Type};
use crate::core::data::stored::StoredData;

impl LiveData for ListLive {
    fn type_tag(&self) -> Type {
        Type::List
    }

    fn as_list(&self) -> Option<ExecResult<ListLive>> {
        Some(Ok(self.clone()))
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