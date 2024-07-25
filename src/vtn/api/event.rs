use axum::extract::Path;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use tracing::{debug, info, trace};
use validator::Validate;

use openadr::wire::event::EventContent;
use openadr::wire::event::EventId;
use openadr::wire::program::ProgramId;
use openadr::wire::target::TargetLabel;
use openadr::wire::Event;

use crate::api::{AppResponse, ValidatedQuery};
use crate::data_source::{Crud, EventPostgresSource};
use crate::error::AppError;

pub async fn get_all(
    events: EventPostgresSource,
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
) -> AppResponse<Vec<Event>> {
    Ok(Json(
        events
            .retrieve_all(&query_params)
            .await
            .inspect(|_| trace!(?query_params, "successfully got all events"))
            .inspect_err(|err| debug!(?query_params, ?err, "failed to get events"))?,
    ))
}

pub async fn get(events: EventPostgresSource, Path(id): Path<EventId>) -> AppResponse<Event> {
    Ok(Json(
        events
            .retrieve(&id)
            .await
            .inspect(|_| trace!(event.id=%id, "successfully got event"))
            .inspect_err(|err| debug!(event.id=%id, ?err, "failed to get event"))?,
    ))
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
    events: EventPostgresSource,
    Path(id): Path<EventId>,
    Json(content): Json<EventContent>,
) -> AppResponse<Event> {
    Ok(Json(
        events
            .update(&id, &content)
            .await
            .inspect(|_| info!(event.id=%id, event_name=?content.event_name, "event updated"))
            .inspect_err(
                |err| debug!(event.id=%id, event_name=?content.event_name, ?err, "event update failed"),
            )?,
    ))
}

pub async fn delete(events: EventPostgresSource, Path(id): Path<EventId>) -> AppResponse<Event> {
    Ok(Json(
        events
            .delete(&id)
            .await
            .inspect(|event| info!(%event.id, event_name=event.content.event_name, "deleted event"))
            .inspect_err(|err| debug!(event.id=%id, ?err, "failed to delete event"))?,
    ))
}

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QueryParams {
    #[serde(rename = "programID")]
    pub program_id: Option<ProgramId>,
    pub target_type: Option<TargetLabel>,
    pub target_values: Option<Vec<String>>,
    #[serde(default)]
    #[validate(range(min = 0))]
    pub skip: i64,
    // TODO how to interpret limit = 0 and what is the default?
    #[validate(range(min = 1, max = 50))]
    #[serde(default = "get_50")]
    pub limit: i64,
}

fn get_50() -> i64 {
    50
}
