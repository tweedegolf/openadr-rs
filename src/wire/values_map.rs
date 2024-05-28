//! Helper types to realize type values relations

use serde::{Deserialize, Serialize};

/// ValuesMap : Represents one or more values associated with a type. E.g. a type of PRICE contains a single float value.

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValuesMap {
    /// Enumerated or private string signifying the nature of values. E.G. \"PRICE\" indicates value is to be interpreted as a currency.
    #[serde(rename = "type")]
    pub value_type: ValueType,
    /// A list of data points. Most often a singular value such as a price.
    pub values: Vec<Value>,
}

// TODO: Add validation len in 1..=128
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValueType(pub String);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Integer(i64),
    Number(f32),
    Boolean(bool),
    Point(Point),
    String(String),
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Point {
    /// A value on an x axis.
    pub x: f32,
    /// A value on a y axis.
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}
