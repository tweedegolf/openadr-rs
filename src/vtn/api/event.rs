use crate::api::{AppResponse, ValidatedQuery};
use crate::error::AppError;
use crate::error::AppError::NotFound;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use openadr::wire::event::EventContent;
use openadr::wire::event::{EventId, QueryParams};
use openadr::wire::Event;
use std::collections::hash_map::Entry;

pub async fn get_all(
    State(state): State<AppState>,
    // TODO use query params
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
) -> AppResponse<Vec<Event>> {
    Ok(Json(state.events.read().await.values().cloned().collect()))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<EventId>) -> AppResponse<Event> {
    Ok(Json(
        state.events.read().await.get(&id).ok_or(NotFound)?.clone(),
    ))
}

// TODO
//   '409':
//   description: Conflict. Implementation dependent response if event with the same eventName exists.
//   content:
//        application/json:
//        schema:
//        $ref: '#/components/schemas/problem'
pub async fn add(
    State(state): State<AppState>,
    Json(new_event): Json<EventContent>,
) -> Result<(StatusCode, Json<Event>), AppError> {
    let event = Event::new(new_event);
    state
        .events
        .write()
        .await
        .insert(event.id.clone(), event.clone());
    Ok((StatusCode::CREATED, Json(event)))
}

// TODO
//   '409':
//   description: Conflict. Implementation dependent response if event with the same eventName exists.
//   content:
//        application/json:
//        schema:
//        $ref: '#/components/schemas/problem'
pub async fn edit(
    State(state): State<AppState>,
    Path(id): Path<EventId>,
    Json(content): Json<EventContent>,
) -> AppResponse<Event> {
    let mut map = state.events.write().await;
    match map.entry(id) {
        Entry::Occupied(mut entry) => {
            let p = entry.get_mut();
            p.content = content;
            p.modification_date_time = Utc::now();
            Ok(Json(p.clone()))
        }
        Entry::Vacant(_) => Err(NotFound),
    }
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<EventId>) -> AppResponse<Event> {
    match state.events.write().await.remove(&id) {
        None => Err(NotFound),
        Some(removed) => Ok(Json(removed)),
    }
}
