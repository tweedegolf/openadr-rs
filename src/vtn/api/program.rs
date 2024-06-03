use axum::extract::{Path, State};
use axum::Json;

use openadr::wire::program::{ProgramId, QueryParams};
use openadr::wire::Program;

use crate::api::{AppResponse, ValidatedQuery};
use crate::error::AppError::NotFound;
use crate::state::AppState;

pub async fn get_all(
    State(state): State<AppState>,
    // TODO handle query params
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
) -> AppResponse<Vec<Program>> {
    Ok(Json(
        state.programs.read().await.values().cloned().collect(),
    ))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<ProgramId>) -> AppResponse<Program> {
    Ok(Json(
        state
            .programs
            .read()
            .await
            .get(&id)
            .ok_or(NotFound)?
            .clone(),
    ))
}
