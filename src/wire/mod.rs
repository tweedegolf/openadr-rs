/// Wire format definitions for OpenADR endpoints
use serde::{Deserialize, Serialize};
use validator::Validate;

pub use event::Event;
pub use problem::Problem;
pub use program::Program;
pub use report::Report;

pub mod event;
pub mod interval;
mod problem;
pub mod program;
pub mod report;
pub mod target;
pub mod values_map;

// TODO: Replace with real ISO8601 type
/// A ISO 8601 formatted date time
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DateTime(String);

// TODO: Replace with real ISO8601 type
/// A ISO 8601 formatted duration
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Duration(String);

// TODO: Find a nice ISO 4217 crate
/// A currency described as listed in ISO 4217
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Currency {
    Todo,
}

// TODO figure out what this is...
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PayloadType(String);

#[derive(Deserialize, Serialize, Debug, Validate)]
pub struct Pagination {
    #[serde(default)]
    skip: u32,
    // TODO how to interpret limit = 0 and what is the default?
    #[validate(range(max = 50))]
    limit: u8,
}
