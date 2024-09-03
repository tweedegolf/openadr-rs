use crate::api::event::QueryParams;
use crate::data_source::{Crud, EventCrud};
use crate::error::AppError;
use axum::async_trait;
use chrono::Utc;
use openadr_wire::event::{EventContent, EventId};
use openadr_wire::target::TargetLabel;
use openadr_wire::Event;
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

impl EventCrud for RwLock<HashMap<EventId, Event>> {}

pub fn new_event(content: EventContent) -> Event {
    Event {
        id: format!("event-{}", Uuid::new_v4()).parse().unwrap(),
        created_date_time: Utc::now(),
        modification_date_time: Utc::now(),
        content,
    }
}

impl QueryParams {
    pub fn matches(&self, event: &Event) -> Result<bool, AppError> {
        if let Some(program_id) = &self.program_id {
            if &event.content.program_id != program_id {
                return Ok(false);
            }
        }

        if let Some(target_type) = self.target_type.as_ref() {
            match target_type {
                TargetLabel::EventName => {
                    let Some(ref event_name) = event.content.event_name else {
                        return Ok(false);
                    };

                    let Some(target_values) = &self.target_values else {
                        return Err(AppError::BadRequest(
                            "If targetType is specified, targetValues must be specified as well",
                        ));
                    };

                    Ok(target_values.iter().any(|name| name == event_name))
                }
                _ => Err(AppError::NotImplemented(
                    "only filtering by event name is supported",
                )),
            }
        } else {
            Ok(true)
        }
    }
}

#[async_trait]
impl Crud for RwLock<HashMap<EventId, Event>> {
    type Type = Event;
    type Id = EventId;
    type NewType = EventContent;
    type Error = AppError;
    type Filter = QueryParams;

    async fn create(&self, content: Self::NewType) -> Result<Self::Type, Self::Error> {
        let event = new_event(content);
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
