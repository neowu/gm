use super::exception::Exception;
use serde::de;

pub fn from_json<'a, T>(json: &'a str) -> Result<T, Exception>
where
    T: de::Deserialize<'a>,
{
    let result: Result<T, serde_json::Error> = serde_json::from_str(json);
    result.map_err(|err| Exception::new(&format!("failed to deserialize json, error={err}, json={json}")))
}
