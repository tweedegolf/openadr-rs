//! Wire format definitions for OpenADR endpoints
//!
//! The types in this module model the messages sent over the wire in OpenADR 3.0.
//! Most types are originally generated from the OpenAPI specification of OpenADR
//! and manually modified to be more idiomatic.

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
/// A ISO 8601 formatted duration
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Duration(String);

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperatingState {
    Normal,
    Error,
    IdleNormal,
    RunningNormal,
    RunningCurtailed,
    RunningHeightened,
    IdleCurtailed,
    #[serde(rename = "SGD_ERROR_CONDITION")]
    SGDErrorCondition,
    IdleHeightened,
    IdleOptedOut,
    RunningOptedOut,
    #[serde(untagged)]
    Private(String),
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DataQuality {
    /// No known reasons to doubt the data.
    Ok,
    /// The data item is currently unavailable.
    Missing,
    /// The data item has been estimated from other available information.
    Estimated,
    /// The data item is suspected to be bad or is known to be.
    Bad,
    /// An application specific privately defined data quality setting.
    #[serde(untagged)]
    Private(String),
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Attribute {
    /// Describes a single geographic point. Values contains 2 floats, generally
    /// representing longitude and latitude. Demand Response programs may define
    /// their own use of these fields.
    Location,
    /// Describes a geographic area. Application specific data. Demand Response
    /// programs may define their own use of these fields, such as GeoJSON
    /// polygon data.
    Area,
    /// The maximum consumption as a float, in kiloWatts.
    MaxPowerConsumption,
    /// The maximum power the device can export as a float, in kiloWatts.
    MaxPowerExport,
    /// A free-form short description of a VEN or resource.
    Description,
    /// An application specific privately defined attribute.
    #[serde(untagged)]
    Private(String),
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Unit {
    /// Kilowatt-hours (kWh)
    #[serde(rename = "KWH")]
    KWH,
    /// Greenhouse gas emissions (g/kWh)
    #[serde(rename = "GHG")]
    GHG,
    /// Voltage (V)
    Volts,
    /// Current (A)
    Amps,
    /// Temperature (C)
    Celcius,
    /// Temperature (F)
    Fahrenheit,
    /// Percentage (%)
    Percent,
    /// Kilowatts
    #[serde(rename = "KW")]
    KW,
    /// Kilovolt-ampere hours (kVAh)
    #[serde(rename = "KVAH")]
    KVAH,
    /// Kilovolt-amperes reactive hours (kVARh)
    #[serde(rename = "KVARH")]
    KVARH,
    /// Kilovolt-amperes (kVA)
    #[serde(rename = "KVA")]
    KVA,
    /// Kilovolt-amperes reactive (kVAR)
    #[serde(rename = "KVAR")]
    KVAR,
    /// An application specific privately defined unit.
    #[serde(untagged)]
    Private(String),
}

#[cfg(test)]
mod tests {
    use crate::wire::{Attribute, DataQuality, OperatingState, Unit};

    #[test]
    fn test_operating_state_serialization() {
        assert_eq!(
            serde_json::to_string(&OperatingState::SGDErrorCondition).unwrap(),
            r#""SGD_ERROR_CONDITION""#
        );
        assert_eq!(
            serde_json::to_string(&OperatingState::Error).unwrap(),
            r#""ERROR""#
        );
        assert_eq!(
            serde_json::to_string(&OperatingState::Private(String::from("something else")))
                .unwrap(),
            r#""something else""#
        );
        assert_eq!(
            serde_json::from_str::<OperatingState>(r#""NORMAL""#).unwrap(),
            OperatingState::Normal
        );
        assert_eq!(
            serde_json::from_str::<OperatingState>(r#""something else""#).unwrap(),
            OperatingState::Private(String::from("something else"))
        );
    }

    #[test]
    fn test_data_quality_serialization() {
        assert_eq!(serde_json::to_string(&DataQuality::Ok).unwrap(), r#""OK""#);
        assert_eq!(
            serde_json::to_string(&DataQuality::Private(String::from("something else"))).unwrap(),
            r#""something else""#
        );
        assert_eq!(
            serde_json::from_str::<DataQuality>(r#""MISSING""#).unwrap(),
            DataQuality::Missing
        );
        assert_eq!(
            serde_json::from_str::<DataQuality>(r#""something else""#).unwrap(),
            DataQuality::Private(String::from("something else"))
        );
    }

    #[test]
    fn test_attribute_serialization() {
        assert_eq!(
            serde_json::to_string(&Attribute::Area).unwrap(),
            r#""AREA""#
        );
        assert_eq!(
            serde_json::to_string(&Attribute::Private(String::from("something else"))).unwrap(),
            r#""something else""#
        );
        assert_eq!(
            serde_json::from_str::<Attribute>(r#""MAX_POWER_EXPORT""#).unwrap(),
            Attribute::MaxPowerExport
        );
        assert_eq!(
            serde_json::from_str::<Attribute>(r#""something else""#).unwrap(),
            Attribute::Private(String::from("something else"))
        );
    }

    #[test]
    fn test_unit_serialization() {
        assert_eq!(serde_json::to_string(&Unit::KVARH).unwrap(), r#""KVARH""#);
        assert_eq!(
            serde_json::to_string(&Unit::Private(String::from("something else"))).unwrap(),
            r#""something else""#
        );
        assert_eq!(
            serde_json::from_str::<Unit>(r#""CELCIUS""#).unwrap(),
            Unit::Celcius
        );
        assert_eq!(
            serde_json::from_str::<Unit>(r#""something else""#).unwrap(),
            Unit::Private(String::from("something else"))
        );
    }
}
