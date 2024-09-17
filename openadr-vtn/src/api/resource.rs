use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use openadr_wire::ven::VenId;
use reqwest::StatusCode;
use serde::Deserialize;
use tracing::{info, trace};
use validator::{Validate, ValidationError};

use openadr_wire::resource::{Resource, ResourceContent, ResourceId};
use openadr_wire::target::TargetLabel;

use crate::api::{AppResponse, ValidatedJson, ValidatedQuery};
use crate::data_source::ResourceCrud;
use crate::error::AppError;
use crate::jwt::{User, VenManagerUser};

pub async fn get_all(
    State(resource_source): State<Arc<dyn ResourceCrud>>,
    Path(ven_id): Path<VenId>,
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
    VenManagerUser(user): VenManagerUser,
) -> AppResponse<Vec<Resource>> {
    trace!(?query_params);

    let resources = resource_source
        .retrieve_all(ven_id, &query_params, &user)
        .await?;

    Ok(Json(resources))
}

pub async fn get(
    State(resource_source): State<Arc<dyn ResourceCrud>>,
    Path(ven_id): Path<VenId>,
    Path(id): Path<ResourceId>,
    User(user): User,
) -> AppResponse<Resource> {
    let ven = resource_source.retrieve(&id, ven_id, &user).await?;

    Ok(Json(ven))
}

pub async fn add(
    State(resource_source): State<Arc<dyn ResourceCrud>>,
    VenManagerUser(user): VenManagerUser,
    Path(ven_id): Path<VenId>,
    ValidatedJson(new_resource): ValidatedJson<ResourceContent>,
) -> Result<(StatusCode, Json<Resource>), AppError> {
    let ven = resource_source.create(new_resource, ven_id, &user).await?;

    Ok((StatusCode::CREATED, Json(ven)))
}

pub async fn edit(
    State(resource_source): State<Arc<dyn ResourceCrud>>,
    Path(ven_id): Path<VenId>,
    Path(id): Path<ResourceId>,
    VenManagerUser(user): VenManagerUser,
    ValidatedJson(content): ValidatedJson<ResourceContent>,
) -> AppResponse<Resource> {
    let resource = resource_source.update(&id, ven_id, content, &user).await?;

    info!(%resource.id, resource.resource_name=resource.content.resource_name, "resource updated");

    Ok(Json(resource))
}

pub async fn delete(
    State(resource_source): State<Arc<dyn ResourceCrud>>,
    Path(ven_id): Path<VenId>,
    Path(id): Path<ResourceId>,
    VenManagerUser(user): VenManagerUser,
) -> AppResponse<Resource> {
    let resource = resource_source.delete(&id, ven_id, &user).await?;
    info!(%id, "deleted resource");
    Ok(Json(resource))
}

#[derive(Deserialize, Validate, Debug)]
#[validate(schema(function = "validate_target_type_value_pair"))]
#[serde(rename_all = "camelCase")]
pub struct QueryParams {
    pub(crate) target_type: Option<TargetLabel>,
    pub(crate) target_values: Option<Vec<String>>,
    #[serde(default)]
    #[validate(range(min = 0))]
    pub(crate) skip: i64,
    #[validate(range(min = 1, max = 50))]
    #[serde(default = "get_50")]
    pub(crate) limit: i64,
}

fn validate_target_type_value_pair(query: &QueryParams) -> Result<(), ValidationError> {
    if query.target_type.is_some() == query.target_values.is_some() {
        Ok(())
    } else {
        Err(ValidationError::new("targetType and targetValues query parameter must either both be set or not set at the same time."))
    }
}

fn get_50() -> i64 {
    50
}
