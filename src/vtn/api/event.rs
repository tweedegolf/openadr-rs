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
use crate::data_source::{Crud, EventPostgresSource};
use crate::error::AppError;
use crate::error::AppError::NotFound;
use crate::state::AppState;

pub async fn get_all(
    events: EventPostgresSource,
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
) -> AppResponse<Vec<Event>> {
    trace!(?query_params);

    Ok(Json(events.retrieve_all(&query_params).await?))
}

pub async fn get(events: EventPostgresSource, Path(id): Path<EventId>) -> AppResponse<Event> {
    Ok(Json(events.retrieve(&id).await?))
}

pub async fn add(
    events: EventPostgresSource,
    Json(new_event): Json<EventContent>,
) -> Result<(StatusCode, Json<Event>), AppError> {
    let event = events.create(&new_event).await?;

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
    pub program_id: Option<ProgramId>,
    pub target_type: Option<TargetLabel>,
    pub target_values: Option<Vec<String>>,
    #[serde(default)]
    pub skip: i64,
    // TODO how to interpret limit = 0 and what is the default?
    #[validate(range(max = 50))]
    #[serde(default = "get_50")]
    pub limit: i64,
}

fn get_50() -> i64 {
    50
}
