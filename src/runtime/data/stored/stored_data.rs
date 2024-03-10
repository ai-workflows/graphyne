use crate::runtime::data::live::{FloatLive, IntLive, StringLive, ListLive, PointerLive, DictLive, FuncLive, FuncValLive, FuncOpLive};
use crate::runtime::data::live::live_data::{BoolLive, ObjectLive, StaticRefLive, TypeLive};
use crate::runtime::{ExecResult, Type};
use crate::runtime::data::functions::v2::FuncV2;

/// Represents data that is currently being stored in memory.
/// This data must be converted to its live counterpart before it can be used.
#[derive(Debug, Clone, PartialEq)]
pub enum StoredData {
    NullStored,
    IntStored(IntLive),
    FloatStored(FloatLive),
    StringStored(StringLive),
    BoolStored(BoolLive),
    PointerStored(PointerLive),
    ListStored(ListLive),
    DictStored(DictLive),
    FuncStored(FuncLive),
    FuncValStored(FuncValLive),
    FuncOpStored(FuncOpLive),
    TypeStored(TypeLive),
    ObjectStored(ObjectLive),
    StaticRefStored(StaticRefLive),
    FuncV2Stored(FuncV2),
}

impl StoredData {
    pub fn type_of(&self) -> ExecResult<TypeLive> {
        match self {
            StoredData::NullStored => Ok(Type::Null),
            StoredData::IntStored(_) => Ok(Type::Integer),
            StoredData::FloatStored(_) => Ok(Type::Float),
            StoredData::StringStored(_) => Ok(Type::String),
            StoredData::BoolStored(_) => Ok(Type::Boolean),
            StoredData::PointerStored(_) => Ok(Type::Pointer),
            StoredData::ListStored(_) => Ok(Type::List),
            StoredData::DictStored(_) => Ok(Type::Dictionary),
            StoredData::FuncStored(_) => Ok(Type::Function),
            StoredData::FuncValStored(_) => Ok(Type::FunctionVal),
            StoredData::FuncOpStored(_) => Ok(Type::FunctionOp),
            StoredData::TypeStored(_) => Ok(Type::Type),
            StoredData::ObjectStored(obj) => match obj.type_ptr.as_ref() {
                StoredData::TypeStored(t) => Ok(t.clone()),
                _ => Err("Object type is not a type".to_string()),
            },
            StoredData::StaticRefStored(val) => match val.get() {
                Some(data) => data.type_of(),
                None => Err("Static Reference is not initialized.".to_string())
            },
            StoredData::FuncV2Stored(_) => Ok(Type::Function),
        }
    }

    fn match_stored_data<'a, T, F>(&'a self, f: F) -> ExecResult<T>
        where
            F: Fn(&'a StoredData) -> ExecResult<T>,
    {
        match self {
            StoredData::StaticRefStored(val) => match val.get() {
                Some(data) => f(data),
                None => Err("Static Reference is not initialized.".to_string()),
            },
            _ => Err(format!("Unexpected data type: {:?}", self)),
        }
    }

    pub fn stored_as_null(&self) -> ExecResult<()> {
        match self {
            StoredData::NullStored => Ok(()),
            _ => self.match_stored_data(|data| data.stored_as_null()),
        }
    }

    pub fn stored_as_int(&self) -> ExecResult<&IntLive> {
        match self {
            StoredData::IntStored(value) => Ok(value),
            _ => self.match_stored_data(|data| data.stored_as_int()),
        }
    }

    pub fn stored_as_float(&self) -> ExecResult<&FloatLive> {
        match self {
            StoredData::FloatStored(value) => Ok(value),
            _ => self.match_stored_data(|data| data.stored_as_float()),
        }
    }

    pub fn stored_as_string(&self) -> ExecResult<&StringLive> {
        match self {
            StoredData::StringStored(value) => Ok(value),
            _ => self.match_stored_data(|data| data.stored_as_string()),
        }
    }

    pub fn stored_as_bool(&self) -> ExecResult<&BoolLive> {
        match self {
            StoredData::BoolStored(value) => Ok(value),
            _ => self.match_stored_data(|data| data.stored_as_bool()),
        }
    }

    pub fn stored_as_pointer(&self) -> ExecResult<&PointerLive> {
        match self {
            StoredData::PointerStored(value) => Ok(value),
            _ => self.match_stored_data(|data| data.stored_as_pointer()),
        }
    }

    pub fn stored_as_list(&self) -> ExecResult<&ListLive> {
        match self {
            StoredData::ListStored(value) => Ok(value),
            _ => self.match_stored_data(|data| data.stored_as_list()),
        }
    }

    pub fn stored_as_dict(&self) -> ExecResult<&DictLive> {
        match self {
            StoredData::DictStored(value) => Ok(value),
            _ => self.match_stored_data(|data| data.stored_as_dict()),
        }
    }

    pub fn stored_as_func(&self) -> ExecResult<&FuncLive> {
        match self {
            StoredData::FuncStored(value) => Ok(value),
            _ => self.match_stored_data(|data| data.stored_as_func()),
        }
    }

    pub fn stored_as_funcv2(&self) -> ExecResult<&FuncV2> {
        match self {
            StoredData::FuncV2Stored(value) => Ok(value),
            _ => self.match_stored_data(|data| data.stored_as_funcv2()),
        }
    }

    pub fn stored_as_func_val(&self) -> ExecResult<&FuncValLive> {
        match self {
            StoredData::FuncValStored(value) => Ok(value),
            _ => self.match_stored_data(|data| data.stored_as_func_val()),
        }
    }

    pub fn stored_as_func_op(&self) -> ExecResult<&FuncOpLive> {
        match self {
            StoredData::FuncOpStored(value) => Ok(value),
            _ => self.match_stored_data(|data| data.stored_as_func_op()),
        }
    }

    pub fn stored_as_type(&self) -> ExecResult<&TypeLive> {
        match self {
            StoredData::TypeStored(value) => Ok(value),
            _ => self.match_stored_data(|data| data.stored_as_type()),
        }
    }

    pub fn stored_as_object(&self) -> ExecResult<&ObjectLive> {
        match self {
            StoredData::ObjectStored(value) => Ok(value),
            _ => self.match_stored_data(|data| data.stored_as_object()),
        }
    }

    pub fn as_static_ref(&self) -> ExecResult<&StaticRefLive> {
        match self {
            StoredData::StaticRefStored(value) => Ok(value),
            _ => Err(format!("Data is not a static reference: {:?}", self)),
        }
    }
}

// Convert from live data to stored data.
impl From<IntLive> for StoredData {
    fn from(value: IntLive) -> Self {
        StoredData::IntStored(value)
    }
}

impl From<FloatLive> for StoredData {
    fn from(value: FloatLive) -> Self {
        StoredData::FloatStored(value)
    }
}

impl From<StringLive> for StoredData {
    fn from(value: StringLive) -> Self {
        StoredData::StringStored(value)
    }
}

impl From<BoolLive> for StoredData {
    fn from(value: BoolLive) -> Self {
        StoredData::BoolStored(value)
    }
}

impl From<ListLive> for StoredData {
    fn from(value: ListLive) -> Self {
        StoredData::ListStored(value)
    }
}

impl From<PointerLive> for StoredData {
    fn from(value: PointerLive) -> Self {
        StoredData::PointerStored(value)
    }
}

impl From<DictLive> for StoredData {
    fn from(value: DictLive) -> Self {
        StoredData::DictStored(value)
    }
}

impl From<FuncLive> for StoredData {
    fn from(value: FuncLive) -> Self {
        StoredData::FuncStored(value)
    }
}

impl From<FuncValLive> for StoredData {
    fn from(value: FuncValLive) -> Self {
        StoredData::FuncValStored(value)
    }
}

impl From<FuncOpLive> for StoredData {
    fn from(value: FuncOpLive) -> Self {
        StoredData::FuncOpStored(value)
    }
}

impl From<TypeLive> for StoredData {
    fn from(value: TypeLive) -> Self {
        StoredData::TypeStored(value)
    }
}

impl From<ObjectLive> for StoredData {
    fn from(value: ObjectLive) -> Self {
        StoredData::ObjectStored(value)
    }
}