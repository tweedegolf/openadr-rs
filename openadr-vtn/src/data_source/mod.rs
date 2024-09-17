#[cfg(feature = "postgres")]
mod postgres;

use axum::async_trait;
use openadr_wire::{
    event::{EventContent, EventId},
    program::{ProgramContent, ProgramId},
    report::{ReportContent, ReportId},
    resource::{Resource, ResourceContent, ResourceId},
    ven::{Ven, VenContent, VenId},
    Event, Program, Report,
};
use std::sync::Arc;

#[cfg(feature = "postgres")]
pub use postgres::PostgresStorage;

use crate::jwt::Claims;
use crate::{error::AppError, jwt::AuthRole};

#[async_trait]
pub trait Crud: Send + Sync + 'static {
    type Type;
    type Id;
    type NewType;
    type Error;
    type Filter;
    type PermissionFilter;

    async fn create(
        &self,
        new: Self::NewType,
        permission_filter: &Self::PermissionFilter,
    ) -> Result<Self::Type, Self::Error>;
    async fn retrieve(
        &self,
        id: &Self::Id,
        permission_filter: &Self::PermissionFilter,
    ) -> Result<Self::Type, Self::Error>;
    async fn retrieve_all(
        &self,
        filter: &Self::Filter,
        permission_filter: &Self::PermissionFilter,
    ) -> Result<Vec<Self::Type>, Self::Error>;
    async fn update(
        &self,
        id: &Self::Id,
        new: Self::NewType,
        permission_filter: &Self::PermissionFilter,
    ) -> Result<Self::Type, Self::Error>;
    async fn delete(
        &self,
        id: &Self::Id,
        permission_filter: &Self::PermissionFilter,
    ) -> Result<Self::Type, Self::Error>;
}

pub trait ProgramCrud:
    Crud<
    Type = Program,
    Id = ProgramId,
    NewType = ProgramContent,
    Error = AppError,
    Filter = crate::api::program::QueryParams,
    PermissionFilter = Claims,
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
    PermissionFilter = Claims,
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
    PermissionFilter = Claims,
>
{
}

pub trait VenCrud:
    Crud<
    Type = Ven,
    Id = VenId,
    NewType = VenContent,
    Error = AppError,
    Filter = crate::api::ven::QueryParams,
    PermissionFilter = Claims,
>
{
}

pub trait ResourceCrud:
    Crud<
    Type = Resource,
    Id = ResourceId,
    NewType = ResourceContent,
    Error = AppError,
    Filter = crate::api::resource::QueryParams,
    PermissionFilter = Claims,
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
    fn vens(&self) -> Arc<dyn VenCrud>;
    fn resources(&self) -> Arc<dyn ResourceCrud>;
    fn auth(&self) -> Arc<dyn AuthSource>;
}

#[derive(Debug, Clone)]
pub struct AuthInfo {
    pub client_id: String,
    pub roles: Vec<AuthRole>,
}
