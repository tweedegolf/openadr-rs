//! Wire format definitions for OpenADR endpoints
//!
//! The types in this module model the messages sent over the wire in OpenADR 3.0.
//! Most types are originally generated from the OpenAPI specification of OpenADR
//! and manually modified to be more idiomatic.

use std::fmt::Display;

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
        let rfc_str = <&str as Deserialize>::deserialize(deserializer)?;

        match DateTime::parse_from_rfc3339(rfc_str) {
            Ok(datetime) => Ok(datetime.into()),
            Err(_) => Err(serde::de::Error::invalid_value(
                Unexpected::Str(rfc_str),
                &"Invalid RFC3339 string",
            )),
        }
    }
}

/// An ISO 8601 formatted duration
#[derive(Clone, Debug, PartialEq)]
pub struct Duration(::iso8601_duration::Duration);

impl<'de> Deserialize<'de> for Duration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        let duration = raw
            .parse::<::iso8601_duration::Duration>()
            .map_err(|_| "iso8601_duration::ParseDurationError")
            .map_err(serde::de::Error::custom)?;

        Ok(Self(duration))
    }
}

impl Serialize for Duration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl Duration {
    pub const fn hour() -> Self {
        Self(::iso8601_duration::Duration {
            year: 0.0,
            month: 0.0,
            day: 0.0,
            hour: 1.0,
            minute: 0.0,
            second: 0.0,
        })
    }
}

impl std::str::FromStr for Duration {
    type Err = ::iso8601_duration::ParseDurationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let duration = s.parse::<::iso8601_duration::Duration>()?;
        Ok(Self(duration))
    }
}

impl Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ::iso8601_duration::Duration {
            year,
            month,
            day,
            hour,
            minute,
            second,
        } = self.0;

        f.write_fmt(format_args!(
            "P{}Y{}M{}DT{}H{}M{}S",
            year, month, day, hour, minute, second
        ))
    }
}

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

    impl quickcheck::Arbitrary for super::Duration {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            // the iso8601_duration library uses an f32 to store the values, which starts losing
            // precision at 24-bit integers.
            super::Duration(::iso8601_duration::Duration {
                year: (<u32 as quickcheck::Arbitrary>::arbitrary(g) & 0x00FF_FFFF) as f32,
                month: (<u32 as quickcheck::Arbitrary>::arbitrary(g) & 0x00FF_FFFF) as f32,
                day: (<u32 as quickcheck::Arbitrary>::arbitrary(g) & 0x00FF_FFFF) as f32,
                hour: (<u32 as quickcheck::Arbitrary>::arbitrary(g) & 0x00FF_FFFF) as f32,
                minute: (<u32 as quickcheck::Arbitrary>::arbitrary(g) & 0x00FF_FFFF) as f32,
                second: (<u32 as quickcheck::Arbitrary>::arbitrary(g) & 0x00FF_FFFF) as f32,
            })
        }
    }

    #[test]
    fn duration_to_string_from_str_roundtrip() {
        quickcheck::quickcheck(test as fn(_) -> bool);

        fn test(input: super::Duration) -> bool {
            let roundtrip = input.to_string().parse::<super::Duration>().unwrap();

            assert_eq!(input.0, roundtrip.0);

            input.0 == roundtrip.0
        }
    }
}
