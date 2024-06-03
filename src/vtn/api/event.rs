use crate::api::{AppResponse, ValidatedQuery};
use crate::error::AppError::NotFound;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use openadr::wire::event::{EventId, QueryParams};
use openadr::wire::Event;

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
