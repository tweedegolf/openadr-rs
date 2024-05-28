use serde::{Deserialize, Serialize};

use crate::wire::{PayloadType, Unit};
use crate::wire::values_map::ValuesMap;

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
    use super::*;

    #[test]
    fn parses_minimal() {
        let json = r#"{"payloadType":"hello"}"#;
        let expected = ReportDescriptor::new(PayloadType("hello".into()));

        assert_eq!(
            serde_json::from_str::<ReportDescriptor>(json).unwrap(),
            expected
        );
    }
}
