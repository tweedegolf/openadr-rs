use serde::{Deserialize, Serialize};

pub use event::Event;
pub use program::Program;

pub mod event;
pub mod interval;
pub mod program;
pub mod report;
pub mod values_map;

// TODO: Replace with real ISO8601 type
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DateTime(String);

// TODO: Replace with real ISO8601 type
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Duration(String);

// TODO: Replace with values from spec
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Unit {
    Todo,
}

// TODO: Find a nice ISO 4217 crate
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Currency {
    Todo,
}

// TODO figure out what this is...
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PayloadType(String);
