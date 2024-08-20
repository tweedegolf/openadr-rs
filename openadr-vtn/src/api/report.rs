use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{async_trait, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use tokio::sync::RwLock;
use tracing::{info, trace};
use validator::Validate;

use openadr_wire::event::EventId;
use openadr_wire::program::ProgramId;
use openadr_wire::report::{ReportContent, ReportId};
use openadr_wire::Report;

use crate::api::{AppResponse, ValidatedQuery};
use crate::data_source::{Crud, ReportCrud};
use crate::error::AppError;
use crate::jwt::{BusinessUser, User};

impl ReportCrud for RwLock<HashMap<ReportId, Report>> {}

#[async_trait]
impl Crud for RwLock<HashMap<ReportId, Report>> {
    type Type = Report;
    type Id = ReportId;
    type NewType = ReportContent;
    type Error = AppError;
    type Filter = QueryParams;

    async fn create(&self, content: Self::NewType) -> Result<Self::Type, Self::Error> {
        let event = Report::new(content);
        self.write().await.insert(event.id.clone(), event.clone());
        Ok(event)
    }

    async fn retrieve(&self, id: &Self::Id) -> Result<Self::Type, Self::Error> {
        self.read().await.get(id).cloned().ok_or(AppError::NotFound)
    }

    async fn retrieve_all(
        &self,
        query_params: &Self::Filter,
    ) -> Result<Vec<Self::Type>, Self::Error> {
        self.read()
            .await
            .values()
            .filter_map(|event| match query_params.matches(event) {
                Ok(true) => Some(Ok(event.clone())),
                Ok(false) => None,
                Err(err) => Some(Err(err)),
            })
            .skip(query_params.skip as usize)
            .take(query_params.limit as usize)
            .collect::<Result<Vec<_>, AppError>>()
    }

    async fn update(
        &self,
        id: &Self::Id,
        content: Self::NewType,
    ) -> Result<Self::Type, Self::Error> {
        match self.write().await.get_mut(id) {
            Some(occupied) => {
                occupied.content = content;
                occupied.modification_date_time = Utc::now();
                Ok(occupied.clone())
            }
            None => Err(AppError::NotFound),
        }
    }

    async fn delete(&self, id: &Self::Id) -> Result<Self::Type, Self::Error> {
        match self.write().await.remove(id) {
            Some(event) => Ok(event),
            None => Err(AppError::NotFound),
        }
    }
}

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

// TODO
//   '409':
//   description: Conflict. Implementation dependent response if report with the same reportName exists.
//   content:
//        application/json:
//        schema:
//        $ref: '#/components/schemas/problem'
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
