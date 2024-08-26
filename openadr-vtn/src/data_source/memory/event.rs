use crate::api::event::QueryParams;
use crate::data_source::{Crud, EventCrud};
use crate::error::AppError;
use axum::async_trait;
use chrono::Utc;
use openadr_wire::event::{EventContent, EventId};
use openadr_wire::Event;
use std::collections::HashMap;
use tokio::sync::RwLock;

impl EventCrud for RwLock<HashMap<EventId, Event>> {}

#[async_trait]
impl Crud for RwLock<HashMap<EventId, Event>> {
    type Type = Event;
    type Id = EventId;
    type NewType = EventContent;
    type Error = AppError;
    type Filter = QueryParams;

    async fn create(&self, content: Self::NewType) -> Result<Self::Type, Self::Error> {
        let event = Event::new(content);
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
