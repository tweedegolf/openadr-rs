use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Event {
    Simple,
    Price,
    ChargeStateSetpoint,
    DispatchSetpoint,
    DispatchSetpointRelative,
    ControlSetpoint,
    ExportPrice,
    #[serde(rename = "GHG")]
    GHG,
    Curve,
    #[serde(rename = "OLS")]
    OLS,
    ImportCapacitySubscription,
    ImportCapacityReservation,
    ImportCapacityReservationFee,
    ImportCapacityAvailable,
    ImportCapacityAvailablePrice,
    ExportCapacitySubscription,
    ExportCapacityReservation,
    ExportCapacityReservationFee,
    ExportCapacityAvailable,
    ExportCapacityAvailablePrice,
    ImportCapacityLimit,
    ExportCapacityLimit,
    AlertGridEmergency,
    AlertBlackStart,
    AlertPossibleOutage,
    AlertFlexAlert,
    AlertFire,
    AlertFreezing,
    AlertWind,
    AlertTsunami,
    AlertAirQuality,
    AlertOther,
    #[serde(rename = "CTA2045_REBOOT")]
    CTA2045Reboot,
    #[serde(rename = "CTA2045_SET_OVERRIDE_STATUS")]
    CTA2045SetOverrideStatus,
    #[serde(untagged)]
    Private(String),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReportType {
    Reading,
    Usage,
    Demand,
    Setpoint,
    DeltaUsage,
    Baseline,
    OperatingState,
    UpRegulationAvailable,
    DownRegulationAvailable,
    RegulationSetpoint,
    StorageUsableCapacity,
    StorageChargeLevel,
    StorageMaxDischargePower,
    StorageMaxChargePower,
    SimpleLevel,
    UsageForecast,
    StorageDispatchForecast,
    LoadShedDeltaAvailable,
    GenerationDeltaAvailable,
    DataQuality,
    ImportReservationCapacity,
    ImportReservationFee,
    ExportReservationCapacity,
    ExportReservationFee,
    #[serde(untagged)]
    Private(String),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReadingType {
    DirectRead,
    Estimated,
    Summed,
    Mean,
    Peak,
    Forecast,
    Average,
    #[serde(untagged)]
    Private(String),
}

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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResourceName {
    AggregatedReport,
    #[serde(untagged)]
    Private(String),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Target {
    /// A Power Service Location is a utility named specific location in
    /// geography or the distribution system, usually the point of service to a
    /// customer site.
    PowerServiceLocation,
    /// A Service Area is a utility named geographic region.
    ServiceArea,
    /// Targeting a specific group (string).
    Group,
    /// Targeting a specific resource (string).
    ResourceName,
    /// Targeting a specific VEN (string).
    #[serde(rename = "VEN_NAME")]
    VENName,
    /// Targeting a specific event (string).
    EventName,
    /// Targeting a specific program (string).
    ProgramName,
    /// An application specific privately defined target.
    #[serde(untagged)]
    Private(String),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
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
    use super::*;

    #[test]
    fn test_event_serialization() {
        assert_eq!(
            serde_json::to_string(&Event::Simple).unwrap(),
            r#""SIMPLE""#
        );
        assert_eq!(
            serde_json::to_string(&Event::CTA2045Reboot).unwrap(),
            r#""CTA2045_REBOOT""#
        );
        assert_eq!(
            serde_json::from_str::<Event>(r#""GHG""#).unwrap(),
            Event::GHG
        );
        assert_eq!(
            serde_json::from_str::<Event>(r#""something else""#).unwrap(),
            Event::Private(String::from("something else"))
        );
    }

    #[test]
    fn test_report_type_serialization() {
        assert_eq!(
            serde_json::to_string(&ReportType::Baseline).unwrap(),
            r#""BASELINE""#
        );
        assert_eq!(
            serde_json::to_string(&ReportType::RegulationSetpoint).unwrap(),
            r#""REGULATION_SETPOINT""#
        );
        assert_eq!(
            serde_json::to_string(&ReportType::Private(String::from("something else"))).unwrap(),
            r#""something else""#
        );
        assert_eq!(
            serde_json::from_str::<ReportType>(r#""DEMAND""#).unwrap(),
            ReportType::Demand
        );
        assert_eq!(
            serde_json::from_str::<ReportType>(r#""EXPORT_RESERVATION_FEE""#).unwrap(),
            ReportType::ExportReservationFee
        );
        assert_eq!(
            serde_json::from_str::<ReportType>(r#""something else""#).unwrap(),
            ReportType::Private(String::from("something else"))
        );
    }

    #[test]
    fn test_reading_type_serialization() {
        assert_eq!(
            serde_json::to_string(&ReadingType::DirectRead).unwrap(),
            r#""DIRECT_READ""#
        );
        assert_eq!(
            serde_json::to_string(&ReadingType::Private(String::from("something else"))).unwrap(),
            r#""something else""#
        );
        assert_eq!(
            serde_json::from_str::<ReadingType>(r#""AVERAGE""#).unwrap(),
            ReadingType::Average
        );
        assert_eq!(
            serde_json::from_str::<ReadingType>(r#""something else""#).unwrap(),
            ReadingType::Private(String::from("something else"))
        );
    }

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
    fn test_resource_name_serialization() {
        assert_eq!(
            serde_json::to_string(&ResourceName::AggregatedReport).unwrap(),
            r#""AGGREGATED_REPORT""#
        );
        assert_eq!(
            serde_json::to_string(&ResourceName::Private(String::from("something else"))).unwrap(),
            r#""something else""#
        );
        assert_eq!(
            serde_json::from_str::<ResourceName>(r#""AGGREGATED_REPORT""#).unwrap(),
            ResourceName::AggregatedReport
        );
        assert_eq!(
            serde_json::from_str::<ResourceName>(r#""something else""#).unwrap(),
            ResourceName::Private(String::from("something else"))
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
    fn test_target_serialization() {
        assert_eq!(
            serde_json::to_string(&Target::EventName).unwrap(),
            r#""EVENT_NAME""#
        );
        assert_eq!(
            serde_json::to_string(&Target::Private(String::from("something else"))).unwrap(),
            r#""something else""#
        );
        assert_eq!(
            serde_json::from_str::<Target>(r#""VEN_NAME""#).unwrap(),
            Target::VENName
        );
        assert_eq!(
            serde_json::from_str::<Target>(r#""something else""#).unwrap(),
            Target::Private(String::from("something else"))
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
