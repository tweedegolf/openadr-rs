/// Wire format definitions for OpenADR endpoints
use serde::{Deserialize, Serialize};

pub use event::Event;
pub use problem::Problem;
pub use program::Program;
pub use report::Report;

pub mod event;
pub mod interval;
mod problem;
pub mod program;
pub mod report;
pub mod target;
pub mod values_map;

mod serde_rfc3339 {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::de::Unexpected;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S, Tz>(time: &DateTime<Tz>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        Tz: TimeZone,
    {
        serializer.serialize_str(&time.to_rfc3339())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(String::deserialize(deserializer)
            .and_then(|rfc_str| {
                DateTime::parse_from_rfc3339(&rfc_str).map_err(|_| {
                    serde::de::Error::invalid_value(
                        Unexpected::Str(&rfc_str),
                        &"Invalid RFC3339 string",
                    )
                })
            })?
            .into())
    }
}

// TODO: Replace with real ISO8601 type
/// A ISO 8601 formatted date time
// #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
// pub struct DateTime(String);

// TODO: Replace with real ISO8601 type
/// A ISO 8601 formatted duration
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Duration(String);

// TODO: Find a nice ISO 4217 crate
/// A currency described as listed in ISO 4217
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Currency {
    Todo,
}

// TODO figure out what this is...
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PayloadType(String);
