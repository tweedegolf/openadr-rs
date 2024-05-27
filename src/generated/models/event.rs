/*
 * OpenADR 3 API
 *
 * The OpenADR 3 API supports energy retailer to energy customer Demand Response programs. The API includes the following capabilities and operations:  __Manage programs:__  * Create/Update/Delete a program * Search programs  __Manage events:__  * Create/Update/Delete an event * Search events  __Manage reports:__  * Create/Update/Delete a report * Search reports  __Manage subscriptions:__  * Create/Update/Delete subscriptions to programs, events, and reports * Search subscriptions * Subscriptions allows clients to register a callback URL (webhook) to be notified   on the change of state of a resource  __Manage vens:__  * Create/Update/Delete vens and ven resources * Search ven and ven resources  __Manage tokens:__  * Obtain an access token * This endpoint is provided as a convenience and may be neglected in a commercial implementation 
 *
 * The version of the OpenAPI document: 3.0.1
 * Contact: frank@pajaritotech.com
 * Generated by: https://openapi-generator.tech
 */

use serde::{Deserialize, Serialize};
use crate::generated::models::{EventPayloadDescriptor, Interval, IntervalPeriod, ReportDescriptor, ValuesMap};

/// Event : Event object to communicate a Demand Response request to VEN. If intervalPeriod is present, sets start time and duration of intervals.



#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Event {
    /// URL safe VTN assigned object ID.
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// datetime in ISO 8601 format
    #[serde(rename = "createdDateTime", skip_serializing_if = "Option::is_none")]
    pub created_date_time: Option<String>,
    /// datetime in ISO 8601 format
    #[serde(rename = "modificationDateTime", skip_serializing_if = "Option::is_none")]
    pub modification_date_time: Option<String>,
    /// Used as discriminator, e.g. notification.object
    #[serde(rename = "objectType", skip_serializing_if = "Option::is_none")]
    pub object_type: Option<ObjectType>,
    /// URL safe VTN assigned object ID.
    #[serde(rename = "programID")]
    pub program_id: String,
    /// User defined string for use in debugging or User Interface.
    #[serde(rename = "eventName", skip_serializing_if = "Option::is_none")]
    pub event_name: Option<String>,
    /// Relative priority of event. A lower number is a higher priority.
    #[serde(rename = "priority", skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    /// A list of valuesMap objects.
    #[serde(rename = "targets", skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<ValuesMap>>,
    /// A list of reportDescriptor objects. Used to request reports from VEN.
    #[serde(rename = "reportDescriptors", skip_serializing_if = "Option::is_none")]
    pub report_descriptors: Option<Vec<ReportDescriptor>>,
    /// A list of payloadDescriptor objects.
    #[serde(rename = "payloadDescriptors", skip_serializing_if = "Option::is_none")]
    pub payload_descriptors: Option<Vec<EventPayloadDescriptor>>,
    #[serde(rename = "intervalPeriod", skip_serializing_if = "Option::is_none")]
    pub interval_period: Option<Box<IntervalPeriod>>,
    /// A list of interval objects.
    #[serde(rename = "intervals")]
    pub intervals: Vec<Interval>,
}

impl Event {
    /// Event object to communicate a Demand Response request to VEN. If intervalPeriod is present, sets start time and duration of intervals. 
    pub fn new(program_id: String, intervals: Vec<Interval>) -> Event {
        Event {
            id: None,
            created_date_time: None,
            modification_date_time: None,
            object_type: None,
            program_id,
            event_name: None,
            priority: None,
            targets: None,
            report_descriptors: None,
            payload_descriptors: None,
            interval_period: None,
            intervals,
        }
    }
}

/// Used as discriminator, e.g. notification.object
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ObjectType {
    #[serde(rename = "EVENT")]
    Event,
}

impl Default for ObjectType {
    fn default() -> ObjectType {
        Self::Event
    }
}

