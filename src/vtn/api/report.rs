use std::collections::hash_map::Entry;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use tracing::{info, trace, warn};
use validator::Validate;

use openadr::wire::event::EventId;
use openadr::wire::program::ProgramId;
use openadr::wire::report::{ReportContent, ReportId};
use openadr::wire::Report;

use crate::api::{AppResponse, ValidatedQuery};
use crate::error::AppError;
use crate::error::AppError::NotFound;
use crate::state::AppState;

pub async fn get_all(
    State(state): State<AppState>,
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
) -> AppResponse<Vec<Report>> {
    trace!(?query_params);

    let reports = state
        .reports
        .read()
        .await
        .values()
        .filter_map(|report| match query_params.matches(report) {
            Ok(true) => Some(Ok(report.clone())),
            Ok(false) => None,
            Err(err) => Some(Err(err)),
        })
        .skip(query_params.skip as usize)
        .take(query_params.limit as usize)
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(Json(reports))
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
    let mut map = state.reports.write().await;

    if let Some(new_report_name) = &new_report.report_name {
        if let Some((name, id)) = map
            .iter()
            .filter_map(|(_, p)| {
                p.content
                    .report_name
                    .clone()
                    .map(|name| (name, p.id.clone()))
            })
            .find(|(name, _)| name == new_report_name)
        {
            warn!(id=%id, report_name=%name, "Conflicting report_name");
            return Err(AppError::Conflict(format!(
                "Report with id {} has the same name",
                id
            )));
        }
    }

    let report = Report::new(new_report);
    map.insert(report.id.clone(), report.clone());

    info!(%report.id,
        report_name=?report.content.report_name,
        "report created"
    );

    Ok((StatusCode::CREATED, Json(report)))
}

pub async fn edit(
    State(state): State<AppState>,
    Path(id): Path<ReportId>,
    Json(content): Json<ReportContent>,
) -> AppResponse<Report> {
    let mut map = state.reports.write().await;

    if let Some((_, conflict)) = map.iter().find(|(inner_id, p)| {
        id != **inner_id
            && content.report_name.is_some()
            && p.content.report_name == content.report_name
    }) {
        warn!(updated=%id, conflicting=%conflict.id, report_name=?content.report_name, "Conflicting report_name");
        return Err(AppError::Conflict(format!(
            "Report with id {} has the same name",
            conflict.id
        )));
    }

    match map.entry(id) {
        Entry::Occupied(mut entry) => {
            let r = entry.get_mut();
            r.content = content;
            r.modification_date_time = Utc::now();

            info!(%r.id,
                report_name=?r.content.report_name,
                "report updated"
            );

            Ok(Json(r.clone()))
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
        Some(removed) => {
            info!(%id, "delete report");
            Ok(Json(removed))
        }
    }
}

#[derive(Serialize, Deserialize, Validate, Debug)]
#[skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct QueryParams {
    #[serde(rename = "programID")]
    program_id: Option<ProgramId>,
    #[serde(rename = "eventID")]
    event_id: Option<EventId>,
    client_name: Option<String>,
    #[serde(default)]
    skip: u32,
    // TODO how to interpret limit = 0 and what is the default?
    #[validate(range(max = 50))]
    #[serde(default = "get_50")]
    limit: u32,
}

fn get_50() -> u32 {
    50
}

impl QueryParams {
    pub fn matches(&self, report: &Report) -> Result<bool, AppError> {
        if let Some(event_id) = &self.event_id {
            Ok(&report.content.event_id == event_id)
        } else if let Some(client_name) = &self.client_name {
            Ok(&report.content.client_name == client_name)
        } else if let Some(program_id) = &self.program_id {
            Ok(&report.content.program_id == program_id)
        } else {
            Ok(true)
        }
    }
}
