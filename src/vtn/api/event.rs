use std::collections::hash_map::Entry;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use tracing::{info, trace, warn};
use validator::Validate;

use openadr::wire::event::EventContent;
use openadr::wire::event::EventId;
use openadr::wire::program::ProgramId;
use openadr::wire::target::TargetLabel;
use openadr::wire::Event;

use crate::api::{AppResponse, ValidatedQuery};
use crate::error::AppError;
use crate::error::AppError::{NotFound, NotImplemented};
use crate::state::AppState;

pub async fn get_all(
    State(state): State<AppState>,
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
) -> AppResponse<Vec<Event>> {
    trace!(?query_params);

    let events = state
        .events
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
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(Json(events))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<EventId>) -> AppResponse<Event> {
    Ok(Json(
        state.events.read().await.get(&id).ok_or(NotFound)?.clone(),
    ))
}

pub async fn add(
    State(state): State<AppState>,
    Json(new_event): Json<EventContent>,
) -> Result<(StatusCode, Json<Event>), AppError> {
    let mut map = state.events.write().await;

    if let Some(new_event_name) = &new_event.event_name {
        if let Some((name, id)) = map
            .iter()
            .filter_map(|(_, p)| {
                p.content
                    .event_name
                    .clone()
                    .map(|name| (name, p.id.clone()))
            })
            .find(|(name, _)| name == new_event_name)
        {
            warn!(id=%id, event_name=%name, "Conflicting event_name");
            return Err(AppError::Conflict(format!(
                "Event with id {} has the same name",
                id
            )));
        }
    }

    let event = Event::new(new_event);
    map.insert(event.id.clone(), event.clone());

    info!(%event.id,
        event_name=?event.content.event_name,
        "event created"
    );

    Ok((StatusCode::CREATED, Json(event)))
}

pub async fn edit(
    State(state): State<AppState>,
    Path(id): Path<EventId>,
    Json(content): Json<EventContent>,
) -> AppResponse<Event> {
    let mut map = state.events.write().await;

    if let Some((_, conflict)) = map.iter().find(|(inner_id, p)| {
        id != **inner_id
            && content.event_name.is_some()
            && p.content.event_name == content.event_name
    }) {
        warn!(updated=%id, conflicting=%conflict.id, event_name=?content.event_name, "Conflicting event_name");
        return Err(AppError::Conflict(format!(
            "Event with id {} has the same name",
            conflict.id
        )));
    }

    match map.entry(id) {
        Entry::Occupied(mut entry) => {
            let e = entry.get_mut();
            e.content = content;
            e.modification_date_time = Utc::now();

            info!(%e.id,
                event_name=?e.content.event_name,
                "event created"
            );

            Ok(Json(e.clone()))
        }
        Entry::Vacant(_) => Err(NotFound),
    }
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<EventId>) -> AppResponse<Event> {
    match state.events.write().await.remove(&id) {
        None => Err(NotFound),
        Some(removed) => {
            info!(%id, "deleted event");
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
