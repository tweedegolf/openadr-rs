use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use reqwest::StatusCode;
use serde::Deserialize;
use tracing::{info, trace};
use validator::{Validate, ValidationError};

use openadr_wire::{
    target::TargetLabel,
    ven::{Ven, VenContent, VenId},
};

use crate::{
    api::{AppResponse, ValidatedJson, ValidatedQuery},
    data_source::VenCrud,
    error::AppError,
    jwt::{User, VenManagerUser},
};

pub async fn get_all(
    State(ven_source): State<Arc<dyn VenCrud>>,
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
    VenManagerUser(user): VenManagerUser,
) -> AppResponse<Vec<Ven>> {
    trace!(?query_params);

    let vens = ven_source.retrieve_all(&query_params, &user).await?;

    Ok(Json(vens))
}

pub async fn get(
    State(ven_source): State<Arc<dyn VenCrud>>,
    Path(id): Path<VenId>,
    User(user): User,
) -> AppResponse<Ven> {
    if user.is_ven() {
        if !user.ven_ids().iter().any(|vid| vid == id.as_str()) {
            return Err(AppError::Forbidden("User does not have access to this VEN"));
        }
    } else if !user.is_ven_manager() {
        return Err(AppError::Forbidden("User is not a VEN or VEN Manager"));
    }

    let ven = ven_source.retrieve(&id, &user).await?;

    Ok(Json(ven))
}

pub async fn add(
    State(ven_source): State<Arc<dyn VenCrud>>,
    VenManagerUser(user): VenManagerUser,
    ValidatedJson(new_ven): ValidatedJson<VenContent>,
) -> Result<(StatusCode, Json<Ven>), AppError> {
    let ven = ven_source.create(new_ven, &user).await?;

    Ok((StatusCode::CREATED, Json(ven)))
}

pub async fn edit(
    State(ven_source): State<Arc<dyn VenCrud>>,
    Path(id): Path<VenId>,
    VenManagerUser(user): VenManagerUser,
    ValidatedJson(content): ValidatedJson<VenContent>,
) -> AppResponse<Ven> {
    let ven = ven_source.update(&id, content, &user).await?;

    info!(%ven.id, ven.ven_name=ven.content.ven_name, "ven updated");

    Ok(Json(ven))
}

pub async fn delete(
    State(ven_source): State<Arc<dyn VenCrud>>,
    Path(id): Path<VenId>,
    VenManagerUser(user): VenManagerUser,
) -> AppResponse<Ven> {
    let ven = ven_source.delete(&id, &user).await?;
    info!(%id, "deleted ven");
    Ok(Json(ven))
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
