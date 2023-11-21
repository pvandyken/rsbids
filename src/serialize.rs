/// Serialization code for once_cell crate taken from https://github.com/matklad/once_cell/pull/104
use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, Serializer};

use std::fmt;
use std::marker::PhantomData;

use once_cell::sync::OnceCell;

pub fn serialize<T: Serialize, S: Serializer>(
    cell: &OnceCell<T>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match cell.get() {
        Some(val) => serializer.serialize_some(val),
        None => serializer.serialize_none(),
    }
}

struct OnceCellVisitor<T>(PhantomData<*const T>);
impl<'de, T: Deserialize<'de>> Visitor<'de> for OnceCellVisitor<T> {
    type Value = OnceCell<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an OnceCell")
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        Ok(OnceCell::from(T::deserialize(deserializer)?))
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(OnceCell::new())
    }
}

pub fn deserialize<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<OnceCell<T>, D::Error> {
    deserializer.deserialize_option(OnceCellVisitor(PhantomData))
}

