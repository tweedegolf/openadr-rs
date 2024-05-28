use serde::{Deserialize, Serialize};

use crate::wire::{PayloadType, Unit};

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
