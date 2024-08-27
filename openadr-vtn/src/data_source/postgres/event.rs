use crate::api::event::QueryParams;
use crate::data_source::{Crud, EventCrud};
use crate::error::AppError;
use axum::async_trait;
use chrono::{DateTime, Utc};
use openadr_wire::event::{EventContent, EventId, Priority};
use openadr_wire::Event;
use sqlx::PgPool;
use std::str::FromStr;
use tracing::error;

#[async_trait]
impl EventCrud for PgEventStorage {}

pub(crate) struct PgEventStorage {
    db: PgPool,
}

impl From<PgPool> for PgEventStorage {
    fn from(db: PgPool) -> Self {
        Self { db }
    }
}

#[derive(Debug)]
struct PostgresEvent {
    id: String,
    created_date_time: DateTime<Utc>,
    modification_date_time: DateTime<Utc>,
    program_id: String,
    event_name: Option<String>,
    priority: Priority,
    targets: Option<serde_json::Value>,
    report_descriptors: Option<serde_json::Value>,
    payload_descriptors: Option<serde_json::Value>,
    interval_period: Option<serde_json::Value>,
    intervals: serde_json::Value,
}

impl TryFrom<PostgresEvent> for Event {
    type Error = AppError;

    #[tracing::instrument(name = "TryFrom<PostgresEvent> for Event")]
    fn try_from(value: PostgresEvent) -> Result<Self, Self::Error> {
        let targets = match value.targets {
            None => None,
            Some(t) => serde_json::from_value(t)
                .inspect_err(|err| {
                    error!(?err, "Failed to deserialize JSON from DB to `TargetMap`")
                })
                .map_err(AppError::SerdeJsonInternalServerError)?,
        };

        let report_descriptors = match value.report_descriptors {
            None => None,
            Some(t) => serde_json::from_value(t)
                .inspect_err(|err| {
                    error!(
                        ?err,
                        "Failed to deserialize JSON from DB to `Vec<ReportDescriptor>`"
                    )
                })
                .map_err(AppError::SerdeJsonInternalServerError)?,
        };

        let payload_descriptors = match value.payload_descriptors {
            None => None,
            Some(t) => serde_json::from_value(t)
                .inspect_err(|err| {
                    error!(
                        ?err,
                        "Failed to deserialize JSON from DB to `Vec<EventPayloadDescriptor>`"
                    )
                })
                .map_err(AppError::SerdeJsonInternalServerError)?,
        };

        let interval_period = match value.interval_period {
            None => None,
            Some(t) => serde_json::from_value(t)
                .inspect_err(|err| {
                    error!(
                        ?err,
                        "Failed to deserialize JSON from DB to `IntervalPeriod`"
                    )
                })
                .map_err(AppError::SerdeJsonInternalServerError)?,
        };

        Ok(Self {
            id: EventId::from_str(&value.id)?,
            created_date_time: value.created_date_time,
            modification_date_time: value.modification_date_time,
            content: EventContent {
                object_type: Default::default(),
                program_id: value.program_id.parse()?,
                event_name: value.event_name,
                priority: value.priority,
                targets,
                report_descriptors,
                payload_descriptors,
                interval_period,
                intervals: serde_json::from_value(value.intervals)
                    .map_err(AppError::SerdeJsonInternalServerError)?,
            },
        })
    }
}

#[async_trait]
impl Crud for PgEventStorage {
    type Type = Event;
    type Id = EventId;
    type NewType = EventContent;
    type Error = AppError;
    type Filter = QueryParams;

    async fn create(&self, new: Self::NewType) -> Result<Self::Type, Self::Error> {
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
            serde_json::to_value(&new.targets).map_err(AppError::SerdeJsonBadRequest)?,
            serde_json::to_value(&new.report_descriptors).map_err(AppError::SerdeJsonBadRequest)?,
            serde_json::to_value(&new.payload_descriptors).map_err( AppError::SerdeJsonBadRequest)?,
            serde_json::to_value(&new.interval_period).map_err(AppError::SerdeJsonBadRequest)?,
            serde_json::to_value(&new.intervals).map_err(AppError::SerdeJsonBadRequest)?,
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

    async fn update(&self, id: &Self::Id, new: Self::NewType) -> Result<Self::Type, Self::Error> {
        Ok(sqlx::query_as!(
            PostgresEvent,
            r#"
            UPDATE event
            SET modification_date_time = now(),
                program_id = $2,
                event_name = $3,
                priority = $4,
                targets = $5,
                report_descriptors = $6,
                payload_descriptors = $7,
                interval_period = $8,
                intervals = $9
            WHERE id = $1
            RETURNING *
            "#,
            id.as_str(),
            new.program_id.as_str(),
            new.event_name,
            Into::<Option<i64>>::into(new.priority),
            serde_json::to_value(&new.targets).map_err(AppError::SerdeJsonBadRequest)?,
            serde_json::to_value(&new.report_descriptors).map_err(AppError::SerdeJsonBadRequest)?,
            serde_json::to_value(&new.payload_descriptors)
                .map_err(AppError::SerdeJsonBadRequest)?,
            serde_json::to_value(&new.interval_period).map_err(AppError::SerdeJsonBadRequest)?,
            serde_json::to_value(&new.intervals).map_err(AppError::SerdeJsonBadRequest)?,
        )
        .fetch_one(&self.db)
        .await?
        .try_into()?)
    }

    async fn delete(&self, id: &Self::Id) -> Result<Self::Type, Self::Error> {
        Ok(sqlx::query_as!(
            PostgresEvent,
            r#"
            DELETE FROM event WHERE id = $1 RETURNING *
            "#,
            id.as_str()
        )
        .fetch_one(&self.db)
        .await?
        .try_into()?)
    }
}

#[cfg(test)]
#[cfg(feature = "live-db-test")]
mod tests {
    use sqlx::PgPool;

    use crate::api::event::QueryParams;
    use crate::error::AppError;
    use chrono::{DateTime, Duration, Utc};
    use openadr_wire::event::{EventContent, EventInterval, EventType, EventValuesMap};
    use openadr_wire::interval::IntervalPeriod;
    use openadr_wire::target::{TargetEntry, TargetLabel, TargetMap};
    use openadr_wire::values_map::Value;
    use openadr_wire::Event;

    impl Default for QueryParams {
        fn default() -> Self {
            Self {
                program_id: None,
                target_type: None,
                target_values: None,
                skip: 0,
                limit: 50,
            }
        }
    }

    fn event_1() -> Event {
        Event {
            id: "event-1".parse().unwrap(),
            created_date_time: "2024-07-25 08:31:10.776000 +00:00".parse().unwrap(),
            modification_date_time: "2024-07-25 08:31:10.776000 +00:00".parse().unwrap(),
            content: EventContent {
                object_type: Default::default(),
                program_id: "program-1".parse().unwrap(),
                event_name: Some("event-1-name".to_string()),
                priority: Some(4).into(),
                targets: Some(TargetMap(vec![TargetEntry {
                    label: TargetLabel::Group,
                    values: ["group-1".to_string()],
                }])),
                report_descriptors: None,
                payload_descriptors: None,
                interval_period: Some(IntervalPeriod {
                    start: "2023-06-15T09:30:00+00:00".parse().unwrap(),
                    duration: Some("P0Y0M0DT1H0M0S".parse().unwrap()),
                    randomize_start: Some("P0Y0M0DT1H0M0S".parse().unwrap()),
                }),
                intervals: vec![EventInterval {
                    id: 3,
                    interval_period: Some(IntervalPeriod {
                        start: "2023-06-15T09:30:00+00:00".parse().unwrap(),
                        duration: Some("P0Y0M0DT1H0M0S".parse().unwrap()),
                        randomize_start: Some("P0Y0M0DT1H0M0S".parse().unwrap()),
                    }),
                    payloads: vec![EventValuesMap {
                        value_type: EventType::Price,
                        values: vec![Value::Number(0.17)],
                    }],
                }],
            },
        }
    }

    fn event_2() -> Event {
        Event {
            id: "event-2".parse().unwrap(),
            created_date_time: "2024-07-25 08:31:10.776000 +00:00".parse().unwrap(),
            modification_date_time: "2024-07-25 08:31:10.776000 +00:00".parse().unwrap(),
            content: EventContent {
                object_type: Default::default(),
                program_id: "program-2".parse().unwrap(),
                event_name: Some("event-2-name".to_string()),
                priority: None.into(),
                targets: Some(TargetMap(vec![TargetEntry {
                    label: TargetLabel::Private("SOME_TARGET".to_string()),
                    values: ["target-1".to_string()],
                }])),
                report_descriptors: None,
                payload_descriptors: None,
                interval_period: None,
                intervals: vec![EventInterval {
                    id: 3,
                    interval_period: None,
                    payloads: vec![EventValuesMap {
                        value_type: EventType::Private("SOME_PAYLOAD".to_string()),
                        values: vec![Value::String("value".to_string())],
                    }],
                }],
            },
        }
    }

    mod get_all {
        use super::*;
        use crate::data_source::postgres::event::PgEventStorage;
        use crate::data_source::Crud;

        #[sqlx::test(fixtures("programs", "events"))]
        async fn default_get_all(db: PgPool) {
            let repo: PgEventStorage = db.into();
            let events = repo.retrieve_all(&Default::default()).await.unwrap();
            assert_eq!(events.len(), 2);
            assert_eq!(events, vec![event_1(), event_2()]);
        }

        #[sqlx::test(fixtures("programs", "events"))]
        async fn limit_get_all(db: PgPool) {
            let repo: PgEventStorage = db.into();
            let events = repo
                .retrieve_all(&QueryParams {
                    limit: 1,
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events, vec![event_1()]);
        }

        #[sqlx::test(fixtures("programs", "events"))]
        async fn skip_get_all(db: PgPool) {
            let repo: PgEventStorage = db.into();
            let events = repo
                .retrieve_all(&QueryParams {
                    skip: 1,
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events, vec![event_2()]);

            let events = repo
                .retrieve_all(&QueryParams {
                    skip: 20,
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 0);
        }

        #[sqlx::test(fixtures("programs", "events"))]
        async fn filter_target_type_get_all(db: PgPool) {
            let repo: PgEventStorage = db.into();
            let events = repo
                .retrieve_all(&QueryParams {
                    target_type: Some(TargetLabel::Group),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events, vec![event_1()]);

            let events = repo
                .retrieve_all(&QueryParams {
                    target_type: Some(TargetLabel::Private("SOME_TARGET".to_string())),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events, vec![event_2()]);

            let events = repo
                .retrieve_all(&QueryParams {
                    target_type: Some(TargetLabel::ProgramName),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 0);
        }

        #[sqlx::test(fixtures("programs", "events"))]
        async fn filter_target_value_get_all(db: PgPool) {
            let repo: PgEventStorage = db.into();

            let events = repo
                .retrieve_all(&QueryParams {
                    target_values: Some(vec!["group-1".to_string()]),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events, vec![event_1()]);

            let events = repo
                .retrieve_all(&QueryParams {
                    target_values: Some(vec!["not-existent".to_string()]),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 0);
        }

        #[sqlx::test(fixtures("programs", "events"))]
        async fn filter_target_type_and_value_get_all(db: PgPool) {
            let repo: PgEventStorage = db.into();

            let events = repo
                .retrieve_all(&QueryParams {
                    target_type: Some(TargetLabel::Group),
                    target_values: Some(vec!["group-1".to_string()]),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events, vec![event_1()]);

            let events = repo
                .retrieve_all(&QueryParams {
                    target_type: Some(TargetLabel::Private("SOME_TARGET".to_string())),
                    target_values: Some(vec!["target-1".to_string()]),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events, vec![event_2()]);

            let events = repo
                .retrieve_all(&QueryParams {
                    target_type: Some(TargetLabel::Group),
                    target_values: Some(vec!["not-existent".to_string()]),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 0);

            let events = repo
                .retrieve_all(&QueryParams {
                    target_type: Some(TargetLabel::Private("NOT_EXISTENT".to_string())),
                    target_values: Some(vec!["target-1".to_string()]),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 0);

            let events = repo
                .retrieve_all(&QueryParams {
                    target_type: Some(TargetLabel::Group),
                    target_values: Some(vec!["target-1".to_string()]),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 0);
        }

        #[sqlx::test(fixtures("programs", "events"))]
        async fn filter_program_id_get_all(db: PgPool) {
            let repo: PgEventStorage = db.into();

            let events = repo
                .retrieve_all(&QueryParams {
                    program_id: Some("program-1".parse().unwrap()),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events, vec![event_1()]);

            let events = repo
                .retrieve_all(&QueryParams {
                    program_id: Some("program-1".parse().unwrap()),
                    target_type: Some(TargetLabel::Group),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events, vec![event_1()]);

            let events = repo
                .retrieve_all(&QueryParams {
                    program_id: Some("not-existent".parse().unwrap()),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 0);
        }
    }

    mod get {
        use super::*;
        use crate::data_source::postgres::event::PgEventStorage;
        use crate::data_source::Crud;

        #[sqlx::test(fixtures("programs", "events"))]
        async fn get_existing(db: PgPool) {
            let repo: PgEventStorage = db.into();
            let event = repo.retrieve(&"event-1".parse().unwrap()).await.unwrap();
            assert_eq!(event, event_1());
        }

        #[sqlx::test(fixtures("programs", "events"))]
        async fn get_not_existing(db: PgPool) {
            let repo: PgEventStorage = db.into();
            let event = repo.retrieve(&"not-existent".parse().unwrap()).await;
            assert!(matches!(event, Err(AppError::NotFound)));
        }
    }

    mod add {
        use super::*;
        use crate::data_source::postgres::event::PgEventStorage;
        use crate::data_source::Crud;

        #[sqlx::test(fixtures("programs"))]
        async fn add(db: PgPool) {
            let repo: PgEventStorage = db.into();
            let event = repo.create(event_1().content).await.unwrap();
            assert_eq!(event.content, event_1().content);
            assert!(event.created_date_time < Utc::now() + Duration::hours(1));
            assert!(event.created_date_time > Utc::now() - Duration::hours(1));
            assert!(event.modification_date_time < Utc::now() + Duration::hours(1));
            assert!(event.modification_date_time > Utc::now() - Duration::hours(1));
        }

        #[sqlx::test(fixtures("programs", "events"))]
        async fn add_existing_conflict_name(db: PgPool) {
            let repo: PgEventStorage = db.into();
            let event = repo.create(event_1().content).await;
            assert!(event.is_ok());
        }
    }

    mod modify {
        use super::*;
        use crate::data_source::postgres::event::PgEventStorage;
        use crate::data_source::Crud;

        #[sqlx::test(fixtures("programs", "events"))]
        async fn updates_modify_time(db: PgPool) {
            let repo: PgEventStorage = db.into();
            let event = repo
                .update(&"event-1".parse().unwrap(), event_1().content)
                .await
                .unwrap();
            assert_eq!(event.content, event_1().content);
            assert_eq!(
                event.created_date_time,
                "2024-07-25 08:31:10.776000 +00:00"
                    .parse::<DateTime<Utc>>()
                    .unwrap()
            );
            assert!(event.modification_date_time < Utc::now() + Duration::hours(1));
            assert!(event.modification_date_time > Utc::now() - Duration::hours(1));
        }

        #[sqlx::test(fixtures("programs", "events"))]
        async fn update(db: PgPool) {
            let repo: PgEventStorage = db.into();
            let mut updated = event_2().content;
            updated.event_name = Some("updated-name".to_string());
            let event = repo
                .update(&"event-1".parse().unwrap(), updated.clone())
                .await
                .unwrap();
            assert_eq!(event.content, updated);
            let event = repo.retrieve(&"event-1".parse().unwrap()).await.unwrap();
            assert_eq!(event.content, updated);
        }

        #[sqlx::test(fixtures("programs", "events"))]
        async fn update_name_conflict(db: PgPool) {
            let repo: PgEventStorage = db.into();
            let event = repo
                .update(&"event-1".parse().unwrap(), event_2().content)
                .await;
            assert!(event.is_ok());
        }
    }

    mod delete {
        use super::*;
        use crate::data_source::postgres::event::PgEventStorage;
        use crate::data_source::Crud;

        #[sqlx::test(fixtures("programs", "events"))]
        async fn delete_existing(db: PgPool) {
            let repo: PgEventStorage = db.into();
            let event = repo.delete(&"event-1".parse().unwrap()).await.unwrap();
            assert_eq!(event, event_1());

            let event = repo.retrieve(&"event-1".parse().unwrap()).await;
            assert!(matches!(event, Err(AppError::NotFound)));

            let event = repo.retrieve(&"event-2".parse().unwrap()).await.unwrap();
            assert_eq!(event, event_2());
        }

        #[sqlx::test(fixtures("programs", "events"))]
        async fn delete_not_existing(db: PgPool) {
            let repo: PgEventStorage = db.into();
            let event = repo.delete(&"not-existent".parse().unwrap()).await;
            assert!(matches!(event, Err(AppError::NotFound)));
        }
    }
}
