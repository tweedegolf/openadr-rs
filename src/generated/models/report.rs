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
use crate::generated::models::{ReportPayloadDescriptor, ReportResourcesInner};

/// Report : report object.



#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Report {
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
    /// URL safe VTN assigned object ID.
    #[serde(rename = "eventID")]
    pub event_id: String,
    /// User generated identifier; may be VEN ID provisioned during program enrollment.
    #[serde(rename = "clientName")]
    pub client_name: String,
    /// User defined string for use in debugging or User Interface.
    #[serde(rename = "reportName", skip_serializing_if = "Option::is_none")]
    pub report_name: Option<String>,
    /// A list of reportPayloadDescriptors.
    #[serde(rename = "payloadDescriptors", skip_serializing_if = "Option::is_none")]
    pub payload_descriptors: Option<Vec<ReportPayloadDescriptor>>,
    /// A list of objects containing report data for a set of resources.
    #[serde(rename = "resources")]
    pub resources: Vec<ReportResourcesInner>,
}

impl Report {
    /// report object.
    pub fn new(program_id: String, event_id: String, client_name: String, resources: Vec<ReportResourcesInner>) -> Report {
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

/// Used as discriminator, e.g. notification.object
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ObjectType {
    #[serde(rename = "REPORT")]
    Report,
}

impl Default for ObjectType {
    fn default() -> ObjectType {
        Self::Report
    }
}

