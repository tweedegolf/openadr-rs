use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use openadr::wire::event::{EventId, Priority};
use openadr::wire::Event;
use openadr::{EventContent, ProgramId};

use crate::api::event::QueryParams;
use crate::data_source::{Crud, Error};
use crate::state::AppState;

pub struct EventPostgresSource {
    db: PgPool,
}

#[async_trait]
impl FromRequestParts<AppState> for EventPostgresSource {
    type Rejection = ();

    async fn from_request_parts(_: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        Ok(Self {
            db: state.pool.clone(),
        })
    }
}

struct PostgresEvent {
    id: EventId,
    created_date_time: DateTime<Utc>,
    modification_date_time: DateTime<Utc>,
    program_id: ProgramId,
    event_name: Option<String>,
    priority: Priority,
    targets: Option<serde_json::Value>,
    report_descriptors: Option<serde_json::Value>,
    payload_descriptors: Option<serde_json::Value>,
    interval_period: Option<serde_json::Value>,
    intervals: serde_json::Value,
}

impl TryFrom<PostgresEvent> for Event {
    type Error = serde_json::Error;

    fn try_from(value: PostgresEvent) -> Result<Self, Self::Error> {
        let targets = match value.targets {
            None => None,
            Some(t) => serde_json::from_value(t)?,
        };

        let report_descriptors = match value.report_descriptors {
            None => None,
            Some(t) => serde_json::from_value(t)?,
        };

        let payload_descriptors = match value.payload_descriptors {
            None => None,
            Some(t) => serde_json::from_value(t)?,
        };

        let interval_period = match value.interval_period {
            None => None,
            Some(t) => serde_json::from_value(t)?,
        };

        Ok(Self {
            id: value.id,
            created_date_time: value.created_date_time,
            modification_date_time: value.modification_date_time,
            content: EventContent {
                object_type: Default::default(),
                program_id: value.program_id,
                event_name: value.event_name,
                priority: value.priority,
                targets,
                report_descriptors,
                payload_descriptors,
                interval_period,
                intervals: serde_json::from_value(value.intervals)?,
            },
        })
    }
}

impl Crud for EventPostgresSource {
    type Type = Event;
    type Id = EventId;
    type NewType = EventContent;
    type Error = Error;
    type Filter = QueryParams;

    async fn create(&self, new: &Self::NewType) -> Result<Self::Type, Self::Error> {
        Ok(sqlx::query_as!(
            PostgresEvent,
            r#"
            INSERT INTO event (id, created_date_time, modification_date_time, program_id, event_name, priority, targets, report_descriptors, payload_descriptors, interval_period, intervals)  
            VALUES (gen_random_uuid(), now(), now(), $1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
            new.program_id.as_str(),
            new.event_name,
            Into::<Option<i64>>::into(new.priority),
            serde_json::to_value(&new.targets)?,
            serde_json::to_value(&new.report_descriptors)?,
            serde_json::to_value(&new.payload_descriptors)?,
            serde_json::to_value(&new.interval_period)?,
            serde_json::to_value(&new.intervals)?
        )
            .fetch_one(&self.db)
            .await?
            .try_into()?
        )
    }

    async fn retrieve(&self, id: &Self::Id) -> Result<Self::Type, Self::Error> {
        Ok(sqlx::query_as!(
            PostgresEvent,
            r#"
            SELECT * FROM event WHERE id = $1
            "#,
            id.as_str()
        )
        .fetch_one(&self.db)
        .await?
        .try_into()?)
    }

    async fn retrieve_all(&self, filter: &Self::Filter) -> Result<Vec<Self::Type>, Self::Error> {
        Ok(sqlx::query_as!(
            PostgresEvent,
            r#"
            SELECT *
            FROM event
            WHERE ($1::text IS NULL OR program_id like $1)
              AND ($2::text IS NULL OR jsonb_path_query_array(targets, '$[*].type') ? $2)
              AND ($3::text[] IS NULL OR jsonb_path_query_array(targets, '$[*].values[*]') ?| ($3))
            OFFSET $4 LIMIT $5
            "#,
            filter.program_id.clone().map(|p| p.to_string()),
            filter.target_type.clone().map(|t| t.to_string()),
            filter.target_values.as_ref().map(|v| v.as_slice()),
            filter.skip,
            filter.limit
        )
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<_, _>>()?)
    }

    async fn update(&self, id: &Self::Id, new: &Self::NewType) -> Result<Self::Type, Self::Error> {
        todo!()
    }

    async fn delete(&self, id: &Self::Id) -> Result<Self::Type, Self::Error> {
        todo!()
    }
}
