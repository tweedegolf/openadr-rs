use thiserror::Error;

mod event;

pub use event::EventPostgresSource;

pub(crate) trait Crud {
    type Type;
    type Id;
    type NewType;
    type Error;
    type Filter;

    async fn create(&self, new: &Self::NewType) -> Result<Self::Type, Self::Error>;
    async fn retrieve(&self, id: &Self::Id) -> Result<Self::Type, Self::Error>;
    async fn retrieve_all(&self, filter: &Self::Filter) -> Result<Vec<Self::Type>, Self::Error>;
    async fn update(&self, id: &Self::Id, new: &Self::NewType) -> Result<Self::Type, Self::Error>;
    async fn delete(&self, id: &Self::Id) -> Result<Self::Type, Self::Error>;
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
}
