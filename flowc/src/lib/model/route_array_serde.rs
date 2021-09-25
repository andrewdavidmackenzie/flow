use std::fmt;
use std::marker::PhantomData;

use serde::de;
use serde::de::{Deserialize, Deserializer};

use crate::model::route::Route;

pub fn route_or_route_array<'de, D>(deserializer: D) -> Result<Vec<Route>, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrVec(PhantomData<Vec<Route>>);

    impl<'de> de::Visitor<'de> for StringOrVec {
        type Value = Vec<Route>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("Route or list of Routes")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![Route::from(value)])
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
