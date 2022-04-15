use serde_dhall::{SimpleType, SimpleValue};

pub struct Schema {
    pub r#type: SimpleType,
    pub default: SimpleValue,
}
