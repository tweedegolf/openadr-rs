use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use tracing::{info, trace};
use validator::{Validate, ValidationError};

use openadr_wire::event::EventContent;
use openadr_wire::event::EventId;
use openadr_wire::program::ProgramId;
use openadr_wire::target::TargetLabel;
use openadr_wire::Event;

use crate::api::{AppResponse, ValidatedQuery};
use crate::data_source::EventCrud;
use crate::error::AppError;
use crate::error::AppError::NotImplemented;
use crate::jwt::{BusinessUser, User};

pub async fn get_all(
    State(event_source): State<Arc<dyn EventCrud>>,
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
    User(_user): User,
) -> AppResponse<Vec<Event>> {
    trace!(?query_params);

    let events = event_source.retrieve_all(&query_params).await?;

    Ok(Json(events))
}

pub async fn get(
    State(event_source): State<Arc<dyn EventCrud>>,
    Path(id): Path<EventId>,
    User(_user): User,
) -> AppResponse<Event> {
    let event = event_source.retrieve(&id).await?;
    Ok(Json(event))
}

pub async fn add(
    State(event_source): State<Arc<dyn EventCrud>>,
    BusinessUser(_user): BusinessUser,
    Json(new_event): Json<EventContent>,
) -> Result<(StatusCode, Json<Event>), AppError> {
    let event = event_source.create(new_event).await?;

    info!(%event.id, event_name=?event.content.event_name, "event created");

    Ok((StatusCode::CREATED, Json(event)))
}

pub async fn edit(
    State(event_source): State<Arc<dyn EventCrud>>,
    Path(id): Path<EventId>,
    BusinessUser(_user): BusinessUser,
    Json(content): Json<EventContent>,
) -> AppResponse<Event> {
    let event = event_source.update(&id, content).await?;

    info!(%event.id, event_name=?event.content.event_name, "event updated");

    Ok(Json(event))
}

pub async fn delete(
    State(event_source): State<Arc<dyn EventCrud>>,
    Path(id): Path<EventId>,
    BusinessUser(_user): BusinessUser,
) -> AppResponse<Event> {
    let event = event_source.delete(&id).await?;
    info!(%id, "deleted event");
    Ok(Json(event))
}

#[derive(Deserialize, Validate, Debug)]
#[validate(schema(function = "validate_target_type_value_pair"))]
#[serde(rename_all = "camelCase")]
pub struct QueryParams {
    #[serde(rename = "programID")]
    pub(crate) program_id: Option<ProgramId>,
    pub(crate) target_type: Option<TargetLabel>,
    pub(crate) target_values: Option<Vec<String>>,
    #[serde(default)]
    #[validate(range(min = 0))]
    pub(crate) skip: i64,
    // TODO how to interpret limit = 0 and what is the default?
    #[validate(range(min = 1, max = 50))]
    #[serde(default = "get_50")]
    pub(crate) limit: i64,
}

fn validate_target_type_value_pair(query: &QueryParams) -> Result<(), ValidationError> {
    if query.target_type.is_some() == query.target_values.is_some() {
        Ok(())
    } else {
        Err(ValidationError::new("targetType and targetValues query parameter must either both be set or not set at the same time."))
    }
}

fn get_50() -> i64 {
    50
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
                _ => Err(NotImplemented("only filtering by event name is supported")),
            }
        } else {
            Ok(true)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        data_source::PostgresStorage,
        jwt::{AuthRole, JwtManager},
        state::AppState,
    };

    use super::*;
    // for `call`, `oneshot`, and `ready`
    use crate::data_source::DataSource;
    use axum::{
        body::Body,
        http::{self, Request, Response, StatusCode},
        Router,
    };
    use http_body_util::BodyExt;
    use openadr_wire::event::Priority;
    use sqlx::PgPool;
    // for `collect`
    use tower::{Service, ServiceExt};

    fn default_event_content() -> EventContent {
        EventContent {
            object_type: None,
            program_id: ProgramId::new("program-1").unwrap(),
            event_name: Some("event_name".to_string()),
            priority: Priority::MAX,
            report_descriptors: None,
            interval_period: None,
            intervals: vec![],
            payload_descriptors: None,
            targets: None,
        }
    }

    fn event_request(method: http::Method, event: Event, token: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(format!("/events/{}", event.id))
            .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::from(serde_json::to_vec(&event).unwrap()))
            .unwrap()
    }

    async fn state_with_events(
        new_events: Vec<EventContent>,
        db: PgPool,
    ) -> (AppState, Vec<Event>) {
        let store = PostgresStorage::new(db).unwrap();
        let mut events = Vec::new();

        for event in new_events {
            events.push(store.events().create(event.clone()).await.unwrap());
            assert_eq!(events[events.len() - 1].content, event)
        }

        (
            AppState::new(store, JwtManager::from_base64_secret("test").unwrap()),
            events,
        )
    }

    fn get_admin_token_from_state(state: &AppState) -> String {
        state
            .jwt_manager
            .create(
                std::time::Duration::from_secs(3600),
                "admin".to_string(),
                vec![AuthRole::AnyBusiness, AuthRole::UserManager],
            )
            .unwrap()
    }

    #[sqlx::test(fixtures("programs"))]
    async fn get(db: PgPool) {
        let (state, mut events) = state_with_events(vec![default_event_content()], db).await;
        let event = events.remove(0);
        let token = get_admin_token_from_state(&state);
        let app = state.into_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::GET)
                    .uri(format!("/events/{}", event.id))
                    .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let db_event: Event = serde_json::from_slice(&body).unwrap();

        assert_eq!(event, db_event);
    }

    #[sqlx::test(fixtures("programs"))]
    async fn delete(db: PgPool) {
        let event1 = EventContent {
            program_id: ProgramId::new("program-1").unwrap(),
            event_name: Some("event1".to_string()),
            ..default_event_content()
        };
        let event2 = EventContent {
            program_id: ProgramId::new("program-2").unwrap(),
            event_name: Some("event2".to_string()),
            ..default_event_content()
        };
        let event3 = EventContent {
            program_id: ProgramId::new("program-2").unwrap(),
            event_name: Some("event3".to_string()),
            ..default_event_content()
        };

        let (state, events) = state_with_events(vec![event1, event2.clone(), event3], db).await;
        let token = get_admin_token_from_state(&state);
        let mut app = state.into_router();

        let event_id = events[1].id.clone();

        let request = Request::builder()
            .method(http::Method::DELETE)
            .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
            .uri(format!("/events/{event_id}"))
            .body(Body::empty())
            .unwrap();

        let response = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(request)
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let db_event: Event = serde_json::from_slice(&body).unwrap();

        assert_eq!(event2, db_event.content);

        let response = retrieve_all_with_filter_help(&mut app, "", &token).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let programs: Vec<Event> = serde_json::from_slice(&body).unwrap();
        assert_eq!(programs.len(), 2);
    }

    #[sqlx::test(fixtures("programs"))]
    async fn update(db: PgPool) {
        let (state, mut events) = state_with_events(vec![default_event_content()], db).await;
        let event = events.remove(0);
        let token = get_admin_token_from_state(&state);
        let app = state.into_router();

        let response = app
            .oneshot(event_request(http::Method::PUT, event.clone(), &token))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let db_program: Event = serde_json::from_slice(&body).unwrap();

        assert_eq!(event.content, db_program.content);
        assert!(event.modification_date_time < db_program.modification_date_time);
    }

    #[sqlx::test(fixtures("users", "programs"))]
    async fn create_same_name(db: PgPool) {
        let (state, _) = state_with_events(vec![], db).await;
        let token = get_admin_token_from_state(&state);
        let mut app = state.into_router();

        let content = default_event_content();

        let request = Request::builder()
            .method(http::Method::POST)
            .uri("/events")
            .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::from(serde_json::to_vec(dbg!(&content)).unwrap()))
            .unwrap();

        let response = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(request)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let request = Request::builder()
            .method(http::Method::POST)
            .uri("/events")
            .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::from(serde_json::to_vec(&content).unwrap()))
            .unwrap();

        // event names don't need to be unique
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    async fn retrieve_all_with_filter_help(
        app: &mut Router,
        query_params: &str,
        token: &str,
    ) -> Response<Body> {
        let request = Request::builder()
            .method(http::Method::GET)
            .uri(format!("/events?{query_params}"))
            .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::empty())
            .unwrap();

        ServiceExt::<Request<Body>>::ready(app)
            .await
            .unwrap()
            .call(request)
            .await
            .unwrap()
    }

    #[sqlx::test(fixtures("programs"))]
    async fn retrieve_all_with_filter(db: PgPool) {
        let event1 = EventContent {
            program_id: ProgramId::new("program-1").unwrap(),
            event_name: Some("event1".to_string()),
            ..default_event_content()
        };
        let event2 = EventContent {
            program_id: ProgramId::new("program-2").unwrap(),
            event_name: Some("event2".to_string()),
            ..default_event_content()
        };
        let event3 = EventContent {
            program_id: ProgramId::new("program-2").unwrap(),
            event_name: Some("event3".to_string()),
            ..default_event_content()
        };

        let (state, _) = state_with_events(vec![event1, event2, event3], db).await;
        let token = get_admin_token_from_state(&state);
        let mut app = state.into_router();

        // no query params
        let response = retrieve_all_with_filter_help(&mut app, "", &token).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let programs: Vec<Event> = serde_json::from_slice(&body).unwrap();
        assert_eq!(programs.len(), 3);

        // skip
        let response = retrieve_all_with_filter_help(&mut app, "skip=1", &token).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let programs: Vec<Event> = serde_json::from_slice(&body).unwrap();
        assert_eq!(programs.len(), 2);

        // limit
        let response = retrieve_all_with_filter_help(&mut app, "limit=2", &token).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let programs: Vec<Event> = serde_json::from_slice(&body).unwrap();
        assert_eq!(programs.len(), 2);

        // program name
        let response = retrieve_all_with_filter_help(&mut app, "targetType=NONSENSE", &token).await;
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);

        let response = retrieve_all_with_filter_help(
            &mut app,
            "targetType=PROGRAM_NAME&targetValues=program1&targetValues=program2",
            &token,
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);

        let response = retrieve_all_with_filter_help(&mut app, "programID=program1", &token).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let programs: Vec<Event> = serde_json::from_slice(&body).unwrap();
        assert_eq!(programs.len(), 1);
    }
}
