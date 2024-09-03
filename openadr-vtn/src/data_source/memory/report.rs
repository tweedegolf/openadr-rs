use crate::api::report::QueryParams;
use crate::data_source::{Crud, ReportCrud};
use crate::error::AppError;
use axum::async_trait;
use chrono::Utc;
use openadr_wire::report::{ReportContent, ReportId};
use openadr_wire::Report;
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

impl ReportCrud for RwLock<HashMap<ReportId, Report>> {}

pub fn new_report(content: ReportContent) -> Report {
    Report {
        id: format!("report-{}", Uuid::new_v4()).parse().unwrap(),
        created_date_time: Utc::now(),
        modification_date_time: Utc::now(),
        content,
    }
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

#[async_trait]
impl Crud for RwLock<HashMap<ReportId, Report>> {
    type Type = Report;
    type Id = ReportId;
    type NewType = ReportContent;
    type Error = AppError;
    type Filter = QueryParams;

    // TODO
    //   '409':
    //   description: Conflict. Implementation dependent response if report with the same reportName exists.
    //   content:
    //        application/json:
    //        schema:
    //        $ref: '#/components/schemas/problem'
    async fn create(&self, content: Self::NewType) -> Result<Self::Type, Self::Error> {
        let event = new_report(content);
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
