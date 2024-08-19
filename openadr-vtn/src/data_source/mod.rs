use std::{collections::HashMap, sync::Arc};

use axum::async_trait;
use openadr_wire::{
    event::{EventContent, EventId},
    program::{ProgramContent, ProgramId},
    report::{ReportContent, ReportId},
    Event, Program, Report,
};
use thiserror::Error;
use tokio::sync::RwLock;

use crate::{error::AppError, jwt::AuthRole};

#[async_trait]
pub trait Crud: Send + Sync + 'static {
    type Type;
    type Id;
    type NewType;
    type Error;
    type Filter;

    async fn create(&self, new: Self::NewType) -> Result<Self::Type, Self::Error>;
    async fn retrieve(&self, id: &Self::Id) -> Result<Self::Type, Self::Error>;
    async fn retrieve_all(&self, filter: &Self::Filter) -> Result<Vec<Self::Type>, Self::Error>;
    async fn update(&self, id: &Self::Id, new: Self::NewType) -> Result<Self::Type, Self::Error>;
    async fn delete(&self, id: &Self::Id) -> Result<Self::Type, Self::Error>;
}

pub trait ProgramCrud:
    Crud<
    Type = Program,
    Id = ProgramId,
    NewType = ProgramContent,
    Error = AppError,
    Filter = crate::api::program::QueryParams,
>
{
}
pub trait ReportCrud:
    Crud<
    Type = Report,
    Id = ReportId,
    NewType = ReportContent,
    Error = AppError,
    Filter = crate::api::report::QueryParams,
>
{
}
pub trait EventCrud:
    Crud<
    Type = Event,
    Id = EventId,
    NewType = EventContent,
    Error = AppError,
    Filter = crate::api::event::QueryParams,
>
{
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Json(#[from] serde_json::Error),
}

#[async_trait]
pub trait AuthSource: Send + Sync + 'static {
    async fn get_user(&self, client_id: &str, client_secret: &str) -> Option<AuthInfo>;
}

pub trait DataSource: Send + Sync + 'static {
    fn programs(&self) -> Arc<dyn ProgramCrud>;
    fn reports(&self) -> Arc<dyn ReportCrud>;
    fn events(&self) -> Arc<dyn EventCrud>;
    fn auth(&self) -> Arc<dyn AuthSource>;
}

#[derive(Debug, Clone)]
pub struct AuthInfo {
    pub client_id: String,
    pub client_secret: String,
    pub role: AuthRole,
    pub ven: Option<String>,
}

#[derive(Default, Clone)]
pub struct InMemoryStorage {
    pub programs: Arc<RwLock<HashMap<ProgramId, Program>>>,
    pub reports: Arc<RwLock<HashMap<ReportId, Report>>>,
    pub events: Arc<RwLock<HashMap<EventId, Event>>>,
    pub auth: Arc<RwLock<Vec<AuthInfo>>>,
}

impl DataSource for InMemoryStorage {
    fn programs(&self) -> Arc<dyn ProgramCrud> {
        self.programs.clone()
    }

    fn reports(&self) -> Arc<dyn ReportCrud> {
        self.reports.clone()
    }

    fn events(&self) -> Arc<dyn EventCrud> {
        self.events.clone()
    }

    fn auth(&self) -> Arc<dyn AuthSource> {
        self.auth.clone()
    }
}

#[async_trait]
impl AuthSource for RwLock<Vec<AuthInfo>> {
    async fn get_user(&self, client_id: &str, client_secret: &str) -> Option<AuthInfo> {
        self.read()
            .await
            .iter()
            .find(|auth| auth.client_id == client_id && auth.client_secret == client_secret)
            .cloned()
    }
}
