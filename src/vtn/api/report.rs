use axum::extract::{Path, State};
use axum::Json;

use openadr::wire::report::{QueryParams, ReportId};
use openadr::wire::Report;

use crate::api::{AppResponse, ValidatedQuery};
use crate::error::AppError::NotFound;
use crate::state::AppState;

pub async fn get_all(
    State(state): State<AppState>,
    // TODO use query params
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
) -> AppResponse<Vec<Report>> {
    Ok(Json(state.reports.read().await.values().cloned().collect()))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<ReportId>) -> AppResponse<Report> {
    Ok(Json(
        state.reports.read().await.get(&id).ok_or(NotFound)?.clone(),
    ))
}
