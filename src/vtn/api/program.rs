use axum::Json;
use serde::Deserialize;
use validator::Validate;

use openadr::wire::program::ProgramName;
use openadr::wire::Program;
use openadr::Target;

use crate::api::{AppResponse, ValidatedQuery};

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
struct QueryParams {
    target_type: Option<Target>,
    target_values: Option<String>,
    #[serde(default)]
    skip: u32,
    #[validate(range(max = 50))]
    limit: u8,
}

pub async fn get_all(
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
) -> AppResponse<Vec<Program>> {
    Ok(Json(vec![Program::new(ProgramName::new(
        "test program".to_string(),
    ))]))
}
