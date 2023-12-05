use crate::core::data::live::{LiveData, FloatLive, IntLive};
use crate::core::data::stored::StoredData;

mod nodes;
mod core;


fn main() {
    let i: IntLive = 2.into();
    let j: IntLive = 6.into();
    let f: FloatLive = 5.5.into();

    let i_stored: StoredData = StoredData::IntStored(i);
    let j_stored: StoredData = StoredData::IntStored(j);
    let f_stored: StoredData = StoredData::FloatStored(f);

    let result = i_stored.as_live().op_add(&f_stored);
    let result2 = i_stored.as_live().op_add(&j_stored);
    let result3 = f_stored.as_live().op_add(&j_stored);

    println!("{:?}", result);
    println!("{:?}", result2);
    println!("{:?}", result3);

    let zero: IntLive = 0.into();
    let zero_stored: StoredData = StoredData::IntStored(zero);

    let result4 = i_stored.as_live().op_div(&zero_stored);

    println!("{:?}", result4);
}
