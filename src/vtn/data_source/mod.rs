use thiserror::Error;

pub(crate) trait Crud<Type> {
    type Id;
    type NewType;
    type Error;
    type Filter;

    async fn create(&self, new: Self::NewType) -> Result<Type, Self::Error>;
    async fn retrieve(&self, id: &Self::Id) -> Result<Type, Self::Error>;
    async fn retrieve_all(&self, filter: &Self::Filter) -> Result<Vec<Type>, Self::Error>;
    async fn update(&self, id: &Self::Id, new: Self::NewType) -> Result<Type, Self::Error>;
    async fn delete(&self, id: &Self::Id) -> Result<Type, Self::Error>;
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Json(#[from] serde_json::Error),
}
