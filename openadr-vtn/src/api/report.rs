use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::{info, trace};
use validator::Validate;

use openadr_wire::event::EventId;
use openadr_wire::program::ProgramId;
use openadr_wire::report::{ReportContent, ReportId};
use openadr_wire::Report;

use crate::api::{AppResponse, ValidatedJson, ValidatedQuery};
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
    ValidatedJson(new_report): ValidatedJson<ReportContent>,
) -> Result<(StatusCode, Json<Report>), AppError> {
    let report = report_source.create(new_report).await?;

    info!(%report.id, report_name=?report.content.report_name, "report created");

    Ok((StatusCode::CREATED, Json(report)))
}

pub async fn edit(
    State(report_source): State<Arc<dyn ReportCrud>>,
    Path(id): Path<ReportId>,
    User(_user): User,
    ValidatedJson(content): ValidatedJson<ReportContent>,
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
#[serde(rename_all = "camelCase")]
pub struct QueryParams {
    #[serde(rename = "programID")]
    pub(crate) program_id: Option<ProgramId>,
    #[serde(rename = "eventID")]
    pub(crate) event_id: Option<EventId>,
    pub(crate) client_name: Option<String>,
    #[serde(default)]
    pub(crate) skip: i64,
    // TODO how to interpret limit = 0 and what is the default?
    #[validate(range(max = 50))]
    #[serde(default = "get_50")]
    pub(crate) limit: i64,
}

fn get_50() -> i64 {
    50
}
