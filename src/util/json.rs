use std::fmt;

use serde::de;
use serde::Serialize;

pub fn from_json<'a, T>(json: &'a str) -> T
where
    T: de::Deserialize<'a>,
{
    serde_json::from_str(json).unwrap_or_else(|err| panic!("failed to deserialize to json, json={json}, error={err}"))
}

pub fn to_json<T>(object: &T) -> String
where
    T: Serialize + fmt::Debug,
{
    serde_json::to_string(object).unwrap_or_else(|err| panic!("failed to serialize to json, object={object:?}, error{err}"))
}
