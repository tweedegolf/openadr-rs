use std::collections::hash_map::Entry;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;

use openadr::wire::report::QueryParams;
use openadr::wire::report::{ReportContent, ReportId};
use openadr::wire::Report;

use crate::api::{AppResponse, ValidatedQuery};
use crate::error::AppError;
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

// TODO
//   '409':
//   description: Conflict. Implementation dependent response if report with the same reportName exists.
//   content:
//        application/json:
//        schema:
//        $ref: '#/components/schemas/problem'
pub async fn add(
    State(state): State<AppState>,
    Json(new_report): Json<ReportContent>,
) -> Result<(StatusCode, Json<Report>), AppError> {
    let report = Report::new(new_report);
    state
        .reports
        .write()
        .await
        .insert(report.id.clone(), report.clone());
    Ok((StatusCode::CREATED, Json(report)))
}

// TODO
//   '409':
//   description: Conflict. Implementation dependent response if report with the same reportName exists.
//   content:
//        application/json:
//        schema:
//        $ref: '#/components/schemas/problem'
pub async fn edit(
    State(state): State<AppState>,
    Path(id): Path<ReportId>,
    Json(content): Json<ReportContent>,
) -> AppResponse<Report> {
    let mut map = state.reports.write().await;
    match map.entry(id) {
        Entry::Occupied(mut entry) => {
            let p = entry.get_mut();
            p.content = content;
            p.modification_date_time = Utc::now();
            Ok(Json(p.clone()))
        }
        Entry::Vacant(_) => Err(NotFound),
    }
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<ReportId>,
) -> AppResponse<Report> {
    match state.reports.write().await.remove(&id) {
        None => Err(NotFound),
        Some(removed) => Ok(Json(removed)),
    }
}
