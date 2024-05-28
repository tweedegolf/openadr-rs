use serde::{Deserialize, Serialize};

use crate::wire::{Currency, PayloadType, Unit};

/// Contextual information used to interpret event valuesMap values. E.g. a PRICE payload simply
/// contains a price value, an associated descriptor provides necessary context such as units and
/// currency.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventPayloadDescriptor {
    /// Enumerated or private string signifying the nature of values.
    pub payload_type: PayloadType,
    /// Units of measure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<Unit>,
    /// Currency of price payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,
}

impl EventPayloadDescriptor {
    pub fn new(payload_type: PayloadType) -> Self {
        Self {
            payload_type,
            units: None,
            currency: None,
        }
    }
}
