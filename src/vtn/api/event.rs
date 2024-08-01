use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use tracing::{info, trace};
use validator::Validate;

use openadr::wire::event::EventContent;
use openadr::wire::event::EventId;
use openadr::wire::program::ProgramId;
use openadr::wire::target::TargetLabel;
use openadr::wire::Event;

use crate::api::{AppResponse, ValidatedQuery};
use crate::data_source::Crud;
use crate::error::AppError;
use crate::error::AppError::{NotFound, NotImplemented};
use crate::state::AppState;

impl Crud<Event> for AppState {
    type Id = EventId;
    type NewType = EventContent;
    type Error = AppError;
    type Filter = QueryParams;

    async fn create(&self, content: Self::NewType) -> Result<Event, Self::Error> {
        let event = Event::new(content);
        self.events
            .write()
            .await
            .insert(event.id.clone(), event.clone());
        Ok(event)
    }

    async fn retrieve(&self, id: &Self::Id) -> Result<Event, Self::Error> {
        self.events
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or(AppError::NotFound)
    }

    async fn retrieve_all(&self, query_params: &Self::Filter) -> Result<Vec<Event>, Self::Error> {
        self.events
            .read()
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

    async fn update(&self, id: &Self::Id, content: Self::NewType) -> Result<Event, Self::Error> {
        match self.events.write().await.get_mut(id) {
            Some(occupied) => {
                occupied.content = content;
                occupied.modification_date_time = Utc::now();
                Ok(occupied.clone())
            }
            None => Err(AppError::NotFound),
        }
    }

    async fn delete(&self, id: &Self::Id) -> Result<Event, Self::Error> {
        match self.events.write().await.remove(id) {
            Some(event) => Ok(event),
            None => Err(AppError::NotFound),
        }
    }
}

pub async fn get_all(
    State(state): State<AppState>,
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
) -> AppResponse<Vec<Event>> {
    trace!(?query_params);

    let events = <AppState as Crud<Event>>::retrieve_all(&state, &query_params).await?;

    Ok(Json(events))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<EventId>) -> AppResponse<Event> {
    let event = <AppState as Crud<Event>>::retrieve(&state, &id).await?;
    Ok(Json(event))
}

pub async fn add(
    State(state): State<AppState>,
    Json(new_event): Json<EventContent>,
) -> Result<(StatusCode, Json<Event>), AppError> {
    let event = <AppState as Crud<Event>>::create(&state, new_event).await?;

    info!(%event.id, event_name=?event.content.event_name, "event created");

    Ok((StatusCode::CREATED, Json(event)))
}

pub async fn edit(
    State(state): State<AppState>,
    Path(id): Path<EventId>,
    Json(content): Json<EventContent>,
) -> AppResponse<Event> {
    let event = <AppState as Crud<Event>>::update(&state, &id, content).await?;

    info!(%event.id, event_name=?event.content.event_name, "event updated");

    Ok(Json(event))
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<EventId>) -> AppResponse<Event> {
    match state.events.write().await.remove(&id) {
        None => Err(NotFound),
        Some(removed) => {
            info!(%id, "event deleted");
            Ok(Json(removed))
        }
    }
}

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QueryParams {
    #[serde(rename = "programID")]
    program_id: Option<ProgramId>,
    target_type: Option<TargetLabel>,
    target_values: Option<Vec<String>>,
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
    pub fn matches(&self, event: &Event) -> Result<bool, AppError> {
        if let Some(program_id) = &self.program_id {
            Ok(&event.content.program_id == program_id)
        } else if self.target_type.is_some() || self.target_values.is_some() {
            Err(NotImplemented(
                "Filtering by target_type and target_values is not supported",
            ))
        } else {
            Ok(true)
        }
    }
}
