/// Wire format definitions for OpenADR endpoints
use serde::{Deserialize, Serialize};

pub use event::Event;
pub use program::Program;
pub use report::Report;

pub mod event;
pub mod interval;
pub mod program;
pub mod report;
pub mod values_map;

// TODO: Replace with real ISO8601 type
/// A ISO 8601 formatted date time
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DateTime(String);

// TODO: Replace with real ISO8601 type
/// A ISO 8601 formatted duration
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Duration(String);

// TODO: Replace with values from spec
/// A physical unit as described in Table 9 of the OpenADR Definition
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Unit {
    Todo,
}

// TODO: Find a nice ISO 4217 crate
/// A currency described as listed in ISO 4217
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Currency {
    Todo,
}

// TODO figure out what this is...
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PayloadType(String);
