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

use crate::wire::values_map::ValuesMap;

/// Resource : A resource is an energy device or system subject to control by a VEN.

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Resource {
    /// URL safe VTN assigned object ID.
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// datetime in ISO 8601 format
    #[serde(rename = "createdDateTime", skip_serializing_if = "Option::is_none")]
    pub created_date_time: Option<String>,
    /// datetime in ISO 8601 format
    #[serde(
        rename = "modificationDateTime",
        skip_serializing_if = "Option::is_none"
    )]
    pub modification_date_time: Option<String>,
    /// Used as discriminator, e.g. notification.object
    #[serde(rename = "objectType", skip_serializing_if = "Option::is_none")]
    pub object_type: Option<ObjectType>,
    /// User generated identifier, resource may be configured with identifier out-of-band.
    #[serde(rename = "resourceName")]
    #[serde(deserialize_with = "crate::wire::string_within_range_inclusive::<1, 128, _>")]
    pub resource_name: String,
    /// URL safe VTN assigned object ID.
    #[serde(rename = "venID", skip_serializing_if = "Option::is_none")]
    pub ven_id: Option<String>,
    /// A list of valuesMap objects describing attributes.
    #[serde(rename = "attributes", skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<ValuesMap>>,
    /// A list of valuesMap objects describing target criteria.
    #[serde(rename = "targets", skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<ValuesMap>>,
}

impl Resource {
    /// A resource is an energy device or system subject to control by a VEN.
    #[allow(dead_code)]
    pub fn new(resource_name: String) -> Resource {
        Resource {
            id: None,
            created_date_time: None,
            modification_date_time: None,
            object_type: None,
            resource_name,
            ven_id: None,
            attributes: None,
            targets: None,
        }
    }
}

/// Used as discriminator, e.g. notification.object
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ObjectType {
    #[serde(rename = "RESOURCE")]
    Resource,
}

impl Default for ObjectType {
    fn default() -> ObjectType {
        Self::Resource
    }
}
