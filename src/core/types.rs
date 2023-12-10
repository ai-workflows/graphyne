/// Represents the "language" type of a piece of data.
/// Note: there may not be a one-to-one correspondence between this, rust-types, and stored-types.
#[derive(Debug)]
pub enum Type {
    Integer,
    Float,
    String,
    Pointer,
    List,
    Dictionary,
}