use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_dhall::{SimpleType, StaticType};
use std::hash::Hash;

#[allow(unused_imports)]
#[macro_use]
extern crate serde_dhall_typegen_macros;
#[doc(hidden)]
pub use serde_dhall_typegen_macros::*;

#[derive(Debug, Default, Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct OrderedF64(pub ordered_float::OrderedFloat<f64>);

impl OrderedF64 {
    /// Get the value out.
    #[inline]
    pub fn into_inner(self) -> f64 {
        self.0.into_inner()
    }
}

impl StaticType for OrderedF64 {
    fn static_type() -> SimpleType {
        SimpleType::Double
    }
}

impl AsRef<f64> for OrderedF64 {
    #[inline]
    fn as_ref(&self) -> &f64 {
        &self.0
    }
}

impl Serialize for OrderedF64 {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(ser)
    }
}

impl<'de> Deserialize<'de> for OrderedF64 {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        f64::deserialize(de).map(|o| OrderedF64(ordered_float::OrderedFloat(o)))
    }
}
