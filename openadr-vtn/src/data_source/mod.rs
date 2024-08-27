#[cfg(not(feature = "sqlx"))]
mod memory;
#[cfg(feature = "postgres")]
mod postgres;

use axum::async_trait;
use openadr_wire::{
    event::{EventContent, EventId},
    program::{ProgramContent, ProgramId},
    report::{ReportContent, ReportId},
    Event, Program, Report,
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(not(feature = "sqlx"))]
pub use memory::InMemoryStorage;
#[cfg(feature = "postgres")]
pub use postgres::PostgresStorage;

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
    pub roles: Vec<AuthRole>,
}

impl AuthInfo {
    pub fn bl_admin() -> Self {
        Self {
            client_id: "admin".to_string(),
            client_secret: "admin".to_string(),
            roles: vec![AuthRole::AnyBusiness, AuthRole::UserManager],
        }
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
