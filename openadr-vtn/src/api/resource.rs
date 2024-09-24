use std::sync::Arc;

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, Path, State},
    http::request::Parts,
    Json,
};
use openadr_wire::ven::VenId;
use reqwest::StatusCode;
use serde::Deserialize;
use tracing::{info, trace};
use validator::{Validate, ValidationError};

use openadr_wire::{
    resource::{Resource, ResourceContent, ResourceId},
    target::TargetLabel,
};

use crate::{
    api::{AppResponse, ValidatedJson, ValidatedQuery},
    data_source::ResourceCrud,
    error::AppError,
    jwt::{Claims, JwtManager, User},
};

pub struct ResourceUser(Claims);

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for ResourceUser
where
    Arc<JwtManager>: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let User(user_claims) = User::from_request_parts(parts, state).await?;
        let Path(ven_id): Path<VenId> = Path::from_request_parts(parts, state)
            .await
            .map_err(|_| AppError::BadRequest("a valid VEN id is required"))?;

        if user_claims.is_ven_manager() {
            return Ok(ResourceUser(user_claims));
        }

        if user_claims.is_ven() && user_claims.ven_ids().contains(&ven_id) {
            return Ok(ResourceUser(user_claims));
        }

        Err(AppError::Forbidden(
            "User not authorized to access this resource",
        ))
    }
}

pub async fn get_all(
    State(resource_source): State<Arc<dyn ResourceCrud>>,
    Path(ven_id): Path<VenId>,
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
    ResourceUser(user): ResourceUser,
) -> AppResponse<Vec<Resource>> {
    trace!(?query_params);

    let resources = resource_source
        .retrieve_all(ven_id, &query_params, &user)
        .await?;

    Ok(Json(resources))
}

pub async fn get(
    State(resource_source): State<Arc<dyn ResourceCrud>>,
    Path((ven_id, id)): Path<(VenId, ResourceId)>,
    ResourceUser(user): ResourceUser,
) -> AppResponse<Resource> {
    let ven = resource_source.retrieve(&id, ven_id, &user).await?;

    Ok(Json(ven))
}

pub async fn add(
    State(resource_source): State<Arc<dyn ResourceCrud>>,
    ResourceUser(user): ResourceUser,
    Path(ven_id): Path<VenId>,
    ValidatedJson(new_resource): ValidatedJson<ResourceContent>,
) -> Result<(StatusCode, Json<Resource>), AppError> {
    let ven = resource_source.create(new_resource, ven_id, &user).await?;

    Ok((StatusCode::CREATED, Json(ven)))
}

pub async fn edit(
    State(resource_source): State<Arc<dyn ResourceCrud>>,
    Path((ven_id, id)): Path<(VenId, ResourceId)>,
    ResourceUser(user): ResourceUser,
    ValidatedJson(content): ValidatedJson<ResourceContent>,
) -> AppResponse<Resource> {
    let resource = resource_source.update(&id, ven_id, content, &user).await?;

    info!(%resource.id, resource.resource_name=resource.content.resource_name, "resource updated");

    Ok(Json(resource))
}

pub async fn delete(
    State(resource_source): State<Arc<dyn ResourceCrud>>,
    Path((ven_id, id)): Path<(VenId, ResourceId)>,
    ResourceUser(user): ResourceUser,
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
