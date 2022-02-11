use std::fmt;
use std::marker::PhantomData;

use serde::de;
use serde::de::{Deserialize, Deserializer};

use crate::model::datatype::DataType;

pub fn datatype_or_datatype_array<'de, D>(deserializer: D) -> Result<Vec<DataType>, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrVec(PhantomData<Vec<DataType>>);

    impl<'de> de::Visitor<'de> for StringOrVec {
        type Value = Vec<DataType>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("DataType or list of DataTypes")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![DataType::from(value)])
        }

        fn visit_seq<S>(self, visitor: S) -> Result<Self::Value, S::Error>
        where
            S: de::SeqAccess<'de>,
        {
            Deserialize::deserialize(de::value::SeqAccessDeserializer::new(visitor))
        }
    }

    deserializer.deserialize_any(StringOrVec(PhantomData))
}
