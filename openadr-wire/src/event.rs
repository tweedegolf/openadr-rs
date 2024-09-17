//! Types used for the `event/` endpoint

use crate::{
    interval::IntervalPeriod, program::ProgramId, report::ReportDescriptor, target::TargetMap,
    values_map::Value, Identifier, IdentifierError, Unit,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};
use validator::Validate;

/// Event object to communicate a Demand Response request to VEN. If intervalPeriod is present, sets
/// start time and duration of intervals.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    /// URL safe VTN assigned object ID.
    pub id: EventId,
    /// datetime in ISO 8601 format
    #[serde(with = "crate::serde_rfc3339")]
    pub created_date_time: DateTime<Utc>,
    /// datetime in ISO 8601 format
    #[serde(with = "crate::serde_rfc3339")]
    pub modification_date_time: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub content: EventContent,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct EventContent {
    /// Used as discriminator, e.g. notification.object
    // TODO: remove this?
    pub object_type: Option<EventObjectType>,
    /// URL safe VTN assigned object ID.
    #[serde(rename = "programID")]
    pub program_id: ProgramId,
    /// User defined string for use in debugging or User Interface.
    pub event_name: Option<String>,
    /// Relative priority of event. A lower number is a higher priority.
    pub priority: Priority,
    /// A list of valuesMap objects.
    pub targets: Option<TargetMap>,
    /// A list of reportDescriptor objects. Used to request reports from VEN.
    pub report_descriptors: Option<Vec<ReportDescriptor>>,
    /// A list of payloadDescriptor objects.
    pub payload_descriptors: Option<Vec<EventPayloadDescriptor>>,
    /// Defines default start and durations of intervals.
    pub interval_period: Option<IntervalPeriod>,
    /// A list of interval objects.
    pub intervals: Vec<EventInterval>,
}

impl EventContent {
    pub fn new(program_id: ProgramId, intervals: Vec<EventInterval>) -> Self {
        assert!(
            !intervals.is_empty(),
            "`EventContent::new` called with no intervals!"
        );

        Self {
            object_type: None,
            program_id,
            event_name: None,
            priority: Priority::UNSPECIFIED,
            targets: None,
            report_descriptors: None,
            payload_descriptors: None,
            interval_period: None,
            intervals,
        }
    }

    pub fn with_event_name(mut self, event_name: impl ToString) -> Self {
        self.event_name = Some(event_name.to_string());
        self
    }

    pub fn with_priority(self, priority: Priority) -> Self {
        Self { priority, ..self }
    }

    pub fn with_targets(mut self, targets: TargetMap) -> Self {
        self.targets = Some(targets);
        self
    }

    pub fn with_report_descriptors(mut self, report_descriptors: Vec<ReportDescriptor>) -> Self {
        self.report_descriptors = Some(report_descriptors);
        self
    }

    pub fn with_payload_descriptors(
        mut self,
        payload_descriptors: Vec<EventPayloadDescriptor>,
    ) -> Self {
        self.payload_descriptors = Some(payload_descriptors);
        self
    }

    pub fn with_interval_period(mut self, interval_period: IntervalPeriod) -> Self {
        self.interval_period = Some(interval_period);
        self
    }

    pub fn with_intervals(mut self, intervals: Vec<EventInterval>) -> Self {
        self.intervals = intervals;
        self
    }
}

/// URL safe VTN assigned object ID
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct EventId(pub(crate) Identifier);

impl Display for EventId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl EventId {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl FromStr for EventId {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

/// Used as discriminator, e.g. notification.object
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventObjectType {
    #[default]
    Event,
}

/// Relative priority of an event
///
/// 0 indicates the highest priority.
///
/// SPEC ASSUMPTION: [`Self::UNSPECIFIED`] has lower priority then any other value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Priority(Option<u32>);

impl Priority {
    pub const UNSPECIFIED: Self = Self(None);

    pub const MAX: Self = Self(Some(0));
    pub const MIN: Self = Self::UNSPECIFIED;

    pub const fn new(val: u32) -> Self {
        Self(Some(val))
    }
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match (self.0, other.0) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(s), Some(o)) => s.cmp(&o).reverse(),
        }
    }
}

impl From<Option<i64>> for Priority {
    fn from(value: Option<i64>) -> Self {
        Self(value.and_then(|i| i.unsigned_abs().try_into().ok()))
    }
}

impl From<Priority> for Option<i64> {
    fn from(value: Priority) -> Self {
        value.0.map(|u| u.into())
    }
}

/// Contextual information used to interpret event valuesMap values. E.g. a PRICE payload simply
/// contains a price value, an associated descriptor provides necessary context such as units and
/// currency.
#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventPayloadDescriptor {
    /// Enumerated or private string signifying the nature of values.
    pub payload_type: EventType,
    /// Units of measure.
    pub units: Option<Unit>,
    /// Currency of price payload.
    pub currency: Option<Currency>,
}

impl EventPayloadDescriptor {
    pub fn new(payload_type: EventType) -> Self {
        Self {
            payload_type,
            units: None,
            currency: None,
        }
    }
}

// TODO: Find a nice ISO 4217 crate
/// A currency described as listed in ISO 4217
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Currency {
    Todo,
}

/// An object defining a temporal window and a list of valuesMaps. if intervalPeriod present may set
/// temporal aspects of interval or override event.intervalPeriod.
#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventInterval {
    /// A client generated number assigned an interval object. Not a sequence number.
    pub id: i32,
    /// Defines default start and durations of intervals.
    pub interval_period: Option<IntervalPeriod>,
    /// A list of valuesMap objects.
    pub payloads: Vec<EventValuesMap>,
}

impl EventInterval {
    pub fn new(id: i32, payloads: Vec<EventValuesMap>) -> Self {
        Self {
            id,
            interval_period: None,
            payloads,
        }
    }
}

/// Represents one or more values associated with a type. E.g. a type of PRICE contains a single float value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventValuesMap {
    /// Enumerated or private string signifying the nature of values. E.G. \"PRICE\" indicates value is to be interpreted as a currency.
    #[serde(rename = "type")]
    pub value_type: EventType,
    /// A list of data points. Most often a singular value such as a price.
    // TODO: The type of Value is actually defined by value_type
    pub values: Vec<Value>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
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
    #[serde(deserialize_with = "crate::string_within_range_inclusive::<1, 128, _>")]
    Private(String),
}

#[cfg(test)]
mod tests {
    use crate::{values_map::Value, Duration};

    use super::*;

    #[test]
    fn test_event_serialization() {
        assert_eq!(
            serde_json::to_string(&EventType::Simple).unwrap(),
            r#""SIMPLE""#
        );
        assert_eq!(
            serde_json::to_string(&EventType::CTA2045Reboot).unwrap(),
            r#""CTA2045_REBOOT""#
        );
        assert_eq!(
            serde_json::from_str::<EventType>(r#""GHG""#).unwrap(),
            EventType::GHG
        );
        assert_eq!(
            serde_json::from_str::<EventType>(r#""something else""#).unwrap(),
            EventType::Private(String::from("something else"))
        );

        assert!(serde_json::from_str::<EventType>(r#""""#).is_err());
        assert!(serde_json::from_str::<EventType>(&format!("\"{}\"", "x".repeat(129))).is_err());
    }

    #[test]
    fn parse_minimal() {
        let example = r#"{"programID":"foo","intervals":[]}"#;
        assert_eq!(
            serde_json::from_str::<EventContent>(example).unwrap(),
            EventContent {
                object_type: None,
                program_id: ProgramId("foo".parse().unwrap()),
                event_name: None,
                priority: Priority::MIN,
                targets: None,
                report_descriptors: None,
                payload_descriptors: None,
                interval_period: None,
                intervals: vec![],
            }
        );
    }

    #[test]
    fn example_parses() {
        let example = r#"[{
                                    "id": "object-999-foo",
                                    "createdDateTime": "2023-06-15T09:30:00Z",
                                    "modificationDateTime": "2023-06-15T09:30:00Z",
                                    "objectType": "EVENT",
                                    "programID": "object-999",
                                    "eventName": "price event 11-18-2022",
                                    "priority": 0,
                                    "targets": null,
                                    "reportDescriptors": null,
                                    "payloadDescriptors": null,
                                    "intervalPeriod": {
                                      "start": "2023-06-15T09:30:00Z",
                                      "duration": "PT1H",
                                      "randomizeStart": "PT1H"
                                    },
                                    "intervals": [
                                      {
                                        "id": 0,
                                        "intervalPeriod": {
                                          "start": "2023-06-15T09:30:00Z",
                                          "duration": "PT1H",
                                          "randomizeStart": "PT1H"
                                        },
                                        "payloads": [
                                          {
                                            "type": "PRICE",
                                            "values": [
                                              0.17
                                            ]
                                          }
                                        ]
                                      }
                                    ]
                                  }]"#;

        let expected = Event {
            id: EventId("object-999-foo".parse().unwrap()),
            created_date_time: "2023-06-15T09:30:00Z".parse().unwrap(),
            modification_date_time: "2023-06-15T09:30:00Z".parse().unwrap(),
            content: EventContent {
                object_type: Some(EventObjectType::Event),
                program_id: ProgramId("object-999".parse().unwrap()),
                event_name: Some("price event 11-18-2022".into()),
                priority: Priority::MAX,
                targets: Default::default(),
                report_descriptors: None,
                payload_descriptors: None,
                interval_period: Some(IntervalPeriod {
                    start: "2023-06-15T09:30:00Z".parse().unwrap(),
                    duration: Some(Duration::PT1H),
                    randomize_start: Some(Duration::PT1H),
                }),
                intervals: vec![EventInterval {
                    id: 0,
                    interval_period: Some(IntervalPeriod {
                        start: "2023-06-15T09:30:00Z".parse().unwrap(),
                        duration: Some(Duration::PT1H),
                        randomize_start: Some(Duration::PT1H),
                    }),
                    payloads: vec![EventValuesMap {
                        value_type: EventType::Price,
                        values: vec![Value::Number(0.17)],
                    }],
                }],
            },
        };

        assert_eq!(
            serde_json::from_str::<Vec<Event>>(example).unwrap()[0],
            expected
        );
    }
}
