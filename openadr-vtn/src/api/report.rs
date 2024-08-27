use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use tracing::{info, trace};
use validator::Validate;

use openadr_wire::event::EventId;
use openadr_wire::program::ProgramId;
use openadr_wire::report::{ReportContent, ReportId};
use openadr_wire::Report;

use crate::api::{AppResponse, ValidatedQuery};
use crate::data_source::ReportCrud;
use crate::error::AppError;
use crate::jwt::{BusinessUser, User};

pub async fn get_all(
    State(report_source): State<Arc<dyn ReportCrud>>,
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
    User(_user): User,
) -> AppResponse<Vec<Report>> {
    trace!(?query_params);

    let reports = report_source.retrieve_all(&query_params).await?;

    Ok(Json(reports))
}

pub async fn get(
    State(report_source): State<Arc<dyn ReportCrud>>,
    Path(id): Path<ReportId>,
    User(_user): User,
) -> AppResponse<Report> {
    let report: Report = report_source.retrieve(&id).await?;
    Ok(Json(report))
}

pub async fn add(
    State(report_source): State<Arc<dyn ReportCrud>>,
    User(_user): User,
    Json(new_report): Json<ReportContent>,
) -> Result<(StatusCode, Json<Report>), AppError> {
    let report = report_source.create(new_report).await?;

    info!(%report.id, report_name=?report.content.report_name, "report created");

    Ok((StatusCode::CREATED, Json(report)))
}

pub async fn edit(
    State(report_source): State<Arc<dyn ReportCrud>>,
    Path(id): Path<ReportId>,
    User(_user): User,
    Json(content): Json<ReportContent>,
) -> AppResponse<Report> {
    let report = report_source.update(&id, content).await?;

    info!(%report.id, report_name=?report.content.report_name, "report updated");

    Ok(Json(report))
}

pub async fn delete(
    State(report_source): State<Arc<dyn ReportCrud>>,
    BusinessUser(_user): BusinessUser,
    Path(id): Path<ReportId>,
) -> AppResponse<Report> {
    let report = report_source.delete(&id).await?;
    info!(%id, "deleted report");
    Ok(Json(report))
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
    pub(crate) skip: u32,
    // TODO how to interpret limit = 0 and what is the default?
    #[validate(range(max = 50))]
    #[serde(default = "get_50")]
    pub(crate) limit: u32,
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
