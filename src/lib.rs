use serde::{Deserialize, Serialize};

pub mod generated;
pub mod wire;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
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

pub struct Client {
    _client: reqwest::Client,
    _base_url: reqwest::Url,
}

impl Client {
    pub fn new(base_url: impl reqwest::IntoUrl) -> reqwest::Result<Client> {
        let client = reqwest::Client::new();
        Self::with_reqwest(base_url, client)
    }

    pub fn with_reqwest(
        base_url: impl reqwest::IntoUrl,
        client: reqwest::Client,
    ) -> reqwest::Result<Client> {
        Ok(Client {
            _client: client,
            _base_url: base_url.into_url()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
