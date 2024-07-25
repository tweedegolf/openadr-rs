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

impl From<PgPool> for EventPostgresSource {
    fn from(db: PgPool) -> Self {
        Self { db }
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
            serde_json::to_value(&new.targets)?,
            serde_json::to_value(&new.report_descriptors)?,
            serde_json::to_value(&new.payload_descriptors)?,
            serde_json::to_value(&new.interval_period)?,
            serde_json::to_value(&new.intervals)?
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
mod tests {
    use sqlx::PgPool;

    use crate::error::AppError;
    use chrono::{DateTime, Duration, Utc};
    use openadr::wire::event::{EventId, EventInterval, EventType, EventValuesMap};
    use openadr::wire::interval::IntervalPeriod;
    use openadr::wire::target::{TargetEntry, TargetLabel, TargetMap};
    use openadr::wire::values_map::Value;
    use openadr::wire::Event;
    use openadr::{EventContent, ProgramId};

    use crate::api::event::QueryParams;
    use crate::data_source::{Crud, EventPostgresSource};

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
            id: EventId("event-1".to_string()),
            created_date_time: "2024-07-25 08:31:10.776000 +00:00".parse().unwrap(),
            modification_date_time: "2024-07-25 08:31:10.776000 +00:00".parse().unwrap(),
            content: EventContent {
                object_type: Default::default(),
                program_id: ProgramId("program-1".to_string()),
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
            id: EventId("event-2".to_string()),
            created_date_time: "2024-07-25 08:31:10.776000 +00:00".parse().unwrap(),
            modification_date_time: "2024-07-25 08:31:10.776000 +00:00".parse().unwrap(),
            content: EventContent {
                object_type: Default::default(),
                program_id: ProgramId("program-2".to_string()),
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

        #[sqlx::test(fixtures("events"))]
        async fn default_get_all(db: PgPool) {
            let repo: EventPostgresSource = db.into();
            let events = repo.retrieve_all(&Default::default()).await.unwrap();
            assert_eq!(events.len(), 2);
            assert_eq!(events, vec![event_1(), event_2()]);
        }

        #[sqlx::test(fixtures("events"))]
        async fn limit_get_all(db: PgPool) {
            let repo: EventPostgresSource = db.into();
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

        #[sqlx::test(fixtures("events"))]
        async fn skip_get_all(db: PgPool) {
            let repo: EventPostgresSource = db.into();
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

        #[sqlx::test(fixtures("events"))]
        async fn filter_target_type_get_all(db: PgPool) {
            let repo: EventPostgresSource = db.into();
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

        #[sqlx::test(fixtures("events"))]
        async fn filter_target_value_get_all(db: PgPool) {
            let repo: EventPostgresSource = db.into();

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

        #[sqlx::test(fixtures("events"))]
        async fn filter_target_type_and_value_get_all(db: PgPool) {
            let repo: EventPostgresSource = db.into();

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

        #[sqlx::test(fixtures("events"))]
        async fn filter_program_id_get_all(db: PgPool) {
            let repo: EventPostgresSource = db.into();

            let events = repo
                .retrieve_all(&QueryParams {
                    program_id: Some(ProgramId("program-1".to_string())),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events, vec![event_1()]);

            let events = repo
                .retrieve_all(&QueryParams {
                    program_id: Some(ProgramId("program-1".to_string())),
                    target_type: Some(TargetLabel::Group),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events, vec![event_1()]);

            let events = repo
                .retrieve_all(&QueryParams {
                    program_id: Some(ProgramId("not-existent".to_string())),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(events.len(), 0);
        }
    }

    mod get {
        use super::*;

        #[sqlx::test(fixtures("events"))]
        async fn get_existing(db: PgPool) {
            let repo: EventPostgresSource = db.into();
            let event = repo
                .retrieve(&EventId("event-1".to_string()))
                .await
                .unwrap();
            assert_eq!(event, event_1());
        }

        #[sqlx::test(fixtures("events"))]
        async fn get_not_existing(db: PgPool) {
            let repo: EventPostgresSource = db.into();
            let event = repo.retrieve(&EventId("not-existent".to_string())).await;
            assert!(matches!(
                event.map_err(|err| err.into()),
                Err(AppError::NotFound)
            ));
        }
    }

    mod add {
        use super::*;

        #[sqlx::test]
        async fn add(db: PgPool) {
            let repo: EventPostgresSource = db.into();
            let event = repo.create(&event_1().content).await.unwrap();
            assert_eq!(event.content, event_1().content);
            assert!(event.created_date_time < Utc::now() + Duration::hours(1));
            assert!(event.created_date_time > Utc::now() - Duration::hours(1));
            assert!(event.modification_date_time < Utc::now() + Duration::hours(1));
            assert!(event.modification_date_time > Utc::now() - Duration::hours(1));
        }

        #[sqlx::test(fixtures("events"))]
        async fn add_existing_conflict_name(db: PgPool) {
            let repo: EventPostgresSource = db.into();
            let event = repo.create(&event_1().content).await;
            assert!(matches!(
                event.map_err(|err| err.into()),
                Err(AppError::Conflict(_))
            ));
        }
    }

    mod modify {
        use super::*;

        #[sqlx::test(fixtures("events"))]
        async fn updates_modify_time(db: PgPool) {
            let repo: EventPostgresSource = db.into();
            let event = repo
                .update(&EventId("event-1".to_string()), &event_1().content)
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

        #[sqlx::test(fixtures("events"))]
        async fn update(db: PgPool) {
            let repo: EventPostgresSource = db.into();
            let mut updated = event_2().content;
            updated.event_name = Some("updated-name".to_string());
            let event = repo
                .update(&EventId("event-1".to_string()), &updated)
                .await
                .unwrap();
            assert_eq!(event.content, updated);
            let event = repo
                .retrieve(&EventId("event-1".to_string()))
                .await
                .unwrap();
            assert_eq!(event.content, updated);
        }

        #[sqlx::test(fixtures("events"))]
        async fn update_name_conflict(db: PgPool) {
            let repo: EventPostgresSource = db.into();
            let event = repo
                .update(&EventId("event-1".to_string()), &event_2().content)
                .await;
            assert!(matches!(
                event.map_err(|err| err.into()),
                Err(AppError::Conflict(_))
            ));
        }
    }

    mod delete {
        use super::*;

        #[sqlx::test(fixtures("events"))]
        async fn delete_existing(db: PgPool) {
            let repo: EventPostgresSource = db.into();
            let event = repo.delete(&EventId("event-1".to_string())).await.unwrap();
            assert_eq!(event, event_1());

            let event = repo.retrieve(&EventId("event-1".to_string())).await;
            assert!(matches!(
                event.map_err(|err| err.into()),
                Err(AppError::NotFound)
            ));

            let event = repo
                .retrieve(&EventId("event-2".to_string()))
                .await
                .unwrap();
            assert_eq!(event, event_2());
        }

        #[sqlx::test(fixtures("events"))]
        async fn delete_not_existing(db: PgPool) {
            let repo: EventPostgresSource = db.into();
            let event = repo.delete(&EventId("not-existent".to_string())).await;
            assert!(matches!(
                event.map_err(|err| err.into()),
                Err(AppError::NotFound)
            ));
        }
    }
}
