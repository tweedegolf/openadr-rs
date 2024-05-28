use serde::{Deserialize, Serialize};

use crate::wire::{DateTime, PayloadType, Unit};
use crate::wire::event::EventId;
use crate::wire::interval::{Interval, IntervalPeriod};
use crate::wire::program::ProgramId;
use crate::wire::values_map::ValuesMap;

/// report object.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// URL safe VTN assigned object ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<ReportId>,
    /// datetime in ISO 8601 format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_date_time: Option<DateTime>,
    /// datetime in ISO 8601 format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modification_date_time: Option<DateTime>,
    /// Used as discriminator, e.g. notification.object
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_type: Option<ObjectType>,
    /// ID attribute of program object this report is associated with.
    #[serde(rename = "programID")]
    pub program_id: ProgramId,
    /// ID attribute of event object this report is associated with.
    #[serde(rename = "eventID")]
    pub event_id: EventId,
    /// User generated identifier; may be VEN ID provisioned during program enrollment.
    // TODO: handle length validation 1..=128
    pub client_name: String,
    /// User defined string for use in debugging or User Interface.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_name: Option<String>,
    /// A list of reportPayloadDescriptors.
    ///
    /// An optional list of objects that provide context to payload types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_descriptors: Option<Vec<ReportPayloadDescriptor>>,
    /// A list of objects containing report data for a set of resources.
    pub resources: Vec<Resource>,
}

impl Report {
    /// report object.
    pub fn new(
        program_id: ProgramId,
        event_id: EventId,
        client_name: String,
        resources: Vec<Resource>,
    ) -> Report {
        Report {
            id: None,
            created_date_time: None,
            modification_date_time: None,
            object_type: None,
            program_id,
            event_id,
            client_name,
            report_name: None,
            payload_descriptors: None,
            resources,
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
pub struct ReportId(pub String);

/// Used as discriminator, e.g. notification.object
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ObjectType {
    #[default]
    Report,
}

/// Report data associated with a resource.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    /// User generated identifier. A value of AGGREGATED_REPORT indicates an aggregation of more
    /// that one resource's data
    // TODO: handle special name and length validation
    pub resource_name: String,
    /// Defines default start and durations of intervals.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_period: Option<IntervalPeriod>,
    /// A list of interval objects.
    pub intervals: Vec<Interval>,
}

impl Resource {
    /// Report data associated with a resource.
    pub fn new(resource_name: String, intervals: Vec<Interval>) -> Resource {
        Resource {
            resource_name,
            interval_period: None,
            intervals,
        }
    }
}

/// An object that may be used to request a report from a VEN. See OpenADR REST User Guide for
/// detailed description of how configure a report request.
// TODO: replace "-1 means" with proper enum
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportDescriptor {
    /// Enumerated or private string signifying the nature of values.
    pub payload_type: PayloadType,
    /// Enumerated or private string signifying the type of reading.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reading_type: Option<ReadingType>,
    /// Units of measure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<Unit>,
    /// A list of valuesMap objects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<ValuesMap>>,
    /// True if report should aggregate results from all targeted resources. False if report includes results for each resource.
    #[serde(default = "bool_false")]
    pub aggregate: bool,
    /// The interval on which to generate a report. -1 indicates generate report at end of last interval.
    #[serde(default = "neg_one")]
    pub start_interval: i32,
    /// The number of intervals to include in a report. -1 indicates that all intervals are to be included.
    #[serde(default = "neg_one")]
    pub num_intervals: i32,
    /// True indicates report on intervals preceding startInterval. False indicates report on intervals following startInterval (e.g. forecast).
    #[serde(default = "bool_true")]
    pub historical: bool,
    /// Number of intervals that elapse between reports. -1 indicates same as numIntervals.
    #[serde(default = "neg_one")]
    pub frequency: i32,
    /// Number of times to repeat report. 1 indicates generate one report. -1 indicates repeat indefinitely.
    #[serde(default = "pos_one")]
    pub repeat: i32,
}

impl ReportDescriptor {
    /// An object that may be used to request a report from a VEN. See OpenADR REST User Guide for detailed description of how configure a report request.
    pub fn new(payload_type: PayloadType) -> Self {
        Self {
            payload_type,
            reading_type: None,
            units: None,
            targets: None,
            aggregate: false,
            start_interval: -1,
            num_intervals: -1,
            historical: true,
            frequency: -1,
            repeat: 1,
        }
    }
}

fn bool_false() -> bool {
    false
}

fn bool_true() -> bool {
    true
}

fn neg_one() -> i32 {
    -1
}

fn pos_one() -> i32 {
    1
}

/// Contextual information used to interpret report payload values. E.g. a USAGE payload simply
/// contains a usage value, an associated descriptor provides necessary context such as units and
/// data quality.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPayloadDescriptor {
    /// Enumerated or private string signifying the nature of values.
    pub payload_type: PayloadType,
    /// Enumerated or private string signifying the type of reading.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reading_type: Option<ReadingType>,
    /// Units of measure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<Unit>,
    /// A quantification of the accuracy of a set of payload values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<f32>,
    /// A quantification of the confidence in a set of payload values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
}

impl ReportPayloadDescriptor {
    pub fn new(payload_type: PayloadType) -> Self {
        Self {
            payload_type,
            reading_type: None,
            units: None,
            accuracy: None,
            confidence: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReadingType {
    DirectRead,
    Todo,
}

// TODO: Add range checks
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Confidence(u8);

#[cfg(test)]
mod tests {
    use crate::wire::Duration;
    use crate::wire::values_map::{Value, ValueType};

    use super::*;

    #[test]
    fn descriptor_parses_minimal() {
        let json = r#"{"payloadType":"hello"}"#;
        let expected = ReportDescriptor::new(PayloadType("hello".into()));

        assert_eq!(
            serde_json::from_str::<ReportDescriptor>(json).unwrap(),
            expected
        );
    }

    #[test]
    fn parses_minimal_report() {
        let example = r#"{"programID":"p1","eventID":"e1","clientName":"c","resources":[]}"#;
        let expected = Report::new(
            ProgramId("p1".into()),
            EventId("e1".into()),
            "c".into(),
            vec![],
        );

        assert_eq!(serde_json::from_str::<Report>(example).unwrap(), expected);
    }

    #[test]
    fn parses_example() {
        let example = r#"[{
            "id": "object-999",
            "createdDateTime": "2023-06-15T09:30:00Z",
            "modificationDateTime": "2023-06-15T09:30:00Z",
            "objectType": "REPORT",
            "programID": "object-999",
            "eventID": "object-999",
            "clientName": "VEN-999",
            "reportName": "Battery_usage_04112023",
            "payloadDescriptors": null,
            "resources": [
              {
                "resourceName": "RESOURCE-999",
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
                        "values": [0.17]
                      }
                    ]
                  }
                ]
              }
            ]
          }]"#;

        let expected = Report {
            id: Some(ReportId("object-999".into())),
            created_date_time: Some(DateTime("2023-06-15T09:30:00Z".into())),
            modification_date_time: Some(DateTime("2023-06-15T09:30:00Z".into())),
            object_type: Some(ObjectType::Report),
            program_id: ProgramId("object-999".into()),
            event_id: EventId("object-999".into()),
            client_name: "VEN-999".into(),
            report_name: Some("Battery_usage_04112023".into()),
            payload_descriptors: None,
            resources: vec![Resource {
                resource_name: "RESOURCE-999".into(),
                interval_period: Some(IntervalPeriod {
                    start: DateTime("2023-06-15T09:30:00Z".into()),
                    duration: Some(Duration("PT1H".into())),
                    randomize_start: Some(Duration("PT1H".into())),
                }),
                intervals: vec![Interval {
                    id: 0,
                    interval_period: Some(IntervalPeriod {
                        start: DateTime("2023-06-15T09:30:00Z".into()),
                        duration: Some(Duration("PT1H".into())),
                        randomize_start: Some(Duration("PT1H".into())),
                    }),
                    payloads: vec![ValuesMap {
                        value_type: ValueType("PRICE".into()),
                        values: vec![Value::Number(0.17)],
                    }],
                }],
            }],
        };

        assert_eq!(
            serde_json::from_str::<Vec<Report>>(example).unwrap()[0],
            expected
        );
    }
}
