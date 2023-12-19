use serde::de::{Error, Unexpected, Visitor};
use serde::{Deserializer, Serializer};
use std::fmt::Formatter;
use std::marker::PhantomData;

pub trait Validator: Sized {
    const EXPECTED_VALUE: &'static str;

    fn serialize<S>(_version: &(), serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(Self::EXPECTED_VALUE)
    }

    fn deserialize<'de, D>(deserializer: D) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ValidatorVisitor::<Self> {
            phantom_data: PhantomData::default(),
        })
    }
}

#[derive(Default)]
struct ValidatorVisitor<T: Validator> {
    phantom_data: PhantomData<T>,
}

impl<'de, T: Validator> Visitor<'de> for ValidatorVisitor<T> {
    type Value = ();

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str(T::EXPECTED_VALUE)
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if v == T::EXPECTED_VALUE {
            Ok(())
        } else {
            Err(E::invalid_value(Unexpected::Str(v), &T::EXPECTED_VALUE))
        }
    }
}
