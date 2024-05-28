//! Types used for the event/ endpoint

use serde::{Deserialize, Serialize};

use crate::wire::interval::IntervalPeriod;
use crate::wire::program::ProgramId;
use crate::wire::report::ReportDescriptor;
use crate::wire::values_map::Value;
use crate::wire::{Currency, DateTime, TargetMap, Unit};

/// Event object to communicate a Demand Response request to VEN. If intervalPeriod is present, sets
/// start time and duration of intervals.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    /// URL safe VTN assigned object ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<EventId>,
    /// datetime in ISO 8601 format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_date_time: Option<DateTime>,
    /// datetime in ISO 8601 format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modification_date_time: Option<DateTime>,
    /// Used as discriminator, e.g. notification.object
    // TODO: remove this?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_type: Option<EventObjectType>,
    /// URL safe VTN assigned object ID.
    #[serde(rename = "programID")]
    pub program_id: ProgramId,
    /// User defined string for use in debugging or User Interface.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_name: Option<String>,
    /// Relative priority of event. A lower number is a higher priority.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u32>,
    /// A list of valuesMap objects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targets: Option<TargetMap>,
    /// A list of reportDescriptor objects. Used to request reports from VEN.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_descriptors: Option<Vec<ReportDescriptor>>,
    /// A list of payloadDescriptor objects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_descriptors: Option<Vec<EventPayloadDescriptor>>,
    /// Defines default start and durations of intervals.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_period: Option<IntervalPeriod>,
    /// A list of interval objects.
    pub intervals: Vec<EventInterval>,
}

impl Event {
    /// Event object to communicate a Demand Response request to VEN. If intervalPeriod is present, sets start time and duration of intervals.
    pub fn new(program_id: ProgramId, intervals: Vec<EventInterval>) -> Event {
        Event {
            id: None,
            created_date_time: None,
            modification_date_time: None,
            object_type: None,
            program_id,
            event_name: None,
            priority: None,
            targets: Default::default(),
            report_descriptors: None,
            payload_descriptors: None,
            interval_period: None,
            intervals,
        }
    }
}

// TODO enforce constraints:
//     objectID:
//         type: string
//         pattern: /^[a-zA-Z0-9_-]*$/
//         minLength: 1
//         maxLength: 128
//         description: URL safe VTN assigned object ID.
//         example: object-999
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct EventId(pub String);

/// Used as discriminator, e.g. notification.object
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventObjectType {
    #[default]
    Event,
}

/// Contextual information used to interpret event valuesMap values. E.g. a PRICE payload simply
/// contains a price value, an associated descriptor provides necessary context such as units and
/// currency.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventPayloadDescriptor {
    /// Enumerated or private string signifying the nature of values.
    pub payload_type: crate::Event,
    /// Units of measure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<Unit>,
    /// Currency of price payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,
}

impl EventPayloadDescriptor {
    pub fn new(payload_type: crate::Event) -> Self {
        Self {
            payload_type,
            units: None,
            currency: None,
        }
    }
}

/// An object defining a temporal window and a list of valuesMaps. if intervalPeriod present may set
/// temporal aspects of interval or override event.intervalPeriod.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventInterval {
    /// A client generated number assigned an interval object. Not a sequence number.
    pub id: i32,
    /// Defines default start and durations of intervals.
    #[serde(skip_serializing_if = "Option::is_none")]
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventValuesMap {
    /// Enumerated or private string signifying the nature of values. E.G. \"PRICE\" indicates value is to be interpreted as a currency.
    #[serde(rename = "type")]
    pub value_type: crate::Event,
    /// A list of data points. Most often a singular value such as a price.
    // TODO: The type of Value is actually defined by value_type
    pub values: Vec<Value>,
}

#[cfg(test)]
mod tests {
    use crate::wire::values_map::Value;
    use crate::wire::Duration;

    use super::*;

    #[test]
    fn parse_minimal() {
        let example = r#"{"programID":"foo","intervals":[]}"#;
        assert_eq!(
            serde_json::from_str::<Event>(example).unwrap(),
            Event::new(ProgramId("foo".into()), vec![])
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
            id: Some(EventId("object-999-foo".into())),
            created_date_time: Some(DateTime("2023-06-15T09:30:00Z".into())),
            modification_date_time: Some(DateTime("2023-06-15T09:30:00Z".into())),
            object_type: Some(EventObjectType::Event),
            program_id: ProgramId("object-999".into()),
            event_name: Some("price event 11-18-2022".into()),
            priority: Some(0),
            targets: Default::default(),
            report_descriptors: None,
            payload_descriptors: None,
            interval_period: Some(IntervalPeriod {
                start: DateTime("2023-06-15T09:30:00Z".into()),
                duration: Some(Duration("PT1H".into())),
                randomize_start: Some(Duration("PT1H".into())),
            }),
            intervals: vec![EventInterval {
                id: 0,
                interval_period: Some(IntervalPeriod {
                    start: DateTime("2023-06-15T09:30:00Z".into()),
                    duration: Some(Duration("PT1H".into())),
                    randomize_start: Some(Duration("PT1H".into())),
                }),
                payloads: vec![EventValuesMap {
                    value_type: crate::Event::Price,
                    values: vec![Value::Number(0.17)],
                }],
            }],
        };

        assert_eq!(
            serde_json::from_str::<Vec<Event>>(example).unwrap()[0],
            expected
        );
    }
}
