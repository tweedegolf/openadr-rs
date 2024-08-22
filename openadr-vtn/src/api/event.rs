use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{async_trait, Json};
use chrono::Utc;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{info, trace};
use validator::Validate;

use openadr_wire::event::EventContent;
use openadr_wire::event::EventId;
use openadr_wire::program::ProgramId;
use openadr_wire::target::TargetLabel;
use openadr_wire::Event;

use crate::api::{AppResponse, ValidatedQuery};
use crate::data_source::{Crud, EventCrud};
use crate::error::AppError;
use crate::error::AppError::NotImplemented;
use crate::jwt::{BusinessUser, User};

impl EventCrud for RwLock<HashMap<EventId, Event>> {}

#[async_trait]
impl Crud for RwLock<HashMap<EventId, Event>> {
    type Type = Event;
    type Id = EventId;
    type NewType = EventContent;
    type Error = AppError;
    type Filter = QueryParams;

    async fn create(&self, content: Self::NewType) -> Result<Self::Type, Self::Error> {
        let event = Event::new(content);
        self.write().await.insert(event.id.clone(), event.clone());
        Ok(event)
    }

    async fn retrieve(&self, id: &Self::Id) -> Result<Self::Type, Self::Error> {
        self.read().await.get(id).cloned().ok_or(AppError::NotFound)
    }

    async fn retrieve_all(
        &self,
        query_params: &Self::Filter,
    ) -> Result<Vec<Self::Type>, Self::Error> {
        self.read()
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

    async fn update(
        &self,
        id: &Self::Id,
        content: Self::NewType,
    ) -> Result<Self::Type, Self::Error> {
        match self.write().await.get_mut(id) {
            Some(occupied) => {
                occupied.content = content;
                occupied.modification_date_time = Utc::now();
                Ok(occupied.clone())
            }
            None => Err(AppError::NotFound),
        }
    }

    async fn delete(&self, id: &Self::Id) -> Result<Self::Type, Self::Error> {
        match self.write().await.remove(id) {
            Some(event) => Ok(event),
            None => Err(AppError::NotFound),
        }
    }
}

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
        } else if let Some(target_type) = self.target_type.as_ref() {
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
        data_source::{AuthInfo, InMemoryStorage},
        jwt::{AuthRole, JwtManager},
        state::AppState,
    };

    use super::*;
    use axum::{
        body::Body,
        http::{self, Request, Response, StatusCode},
        Router,
    };
    use http_body_util::BodyExt;
    use openadr_wire::event::Priority; // for `collect`
    use tower::{Service, ServiceExt}; // for `call`, `oneshot`, and `ready`

    fn default_content() -> EventContent {
        EventContent {
            object_type: None,
            program_id: ProgramId::new("program_id").unwrap(),
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

    fn state_with_events(events: Vec<Event>) -> AppState {
        let store = InMemoryStorage::default();

        store.auth.try_write().unwrap().push(AuthInfo::bl_admin());

        {
            let mut writer = store.events.try_write().unwrap();
            for event in events {
                writer.insert(event.id.clone(), event);
            }
        }

        AppState::new(store, JwtManager::from_base64_secret("test").unwrap())
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

    #[tokio::test]
    async fn get() {
        let event = Event::new(default_content());
        let event_id = event.id.clone();

        let state = state_with_events(vec![event.clone()]);
        let token = get_admin_token_from_state(&state);
        let app = state.into_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::GET)
                    .uri(format!("/events/{event_id}"))
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

    #[tokio::test]
    async fn delete() {
        let event1 = EventContent {
            program_id: ProgramId::new("program1").unwrap(),
            event_name: Some("event1".to_string()),
            ..default_content()
        };
        let event2 = EventContent {
            program_id: ProgramId::new("program2").unwrap(),
            event_name: Some("event2".to_string()),
            ..default_content()
        };
        let event3 = EventContent {
            program_id: ProgramId::new("program3").unwrap(),
            event_name: Some("event3".to_string()),
            ..default_content()
        };

        let events = vec![
            Event::new(event1),
            Event::new(event2.clone()),
            Event::new(event3),
        ];
        let event_id = events[1].id.clone();

        let state = state_with_events(events);
        let token = get_admin_token_from_state(&state);
        let mut app = state.into_router();

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

    #[tokio::test]
    async fn update() {
        let event = Event::new(default_content());

        let state = state_with_events(vec![event.clone()]);
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

    #[tokio::test]
    async fn create_same_name() {
        let state = state_with_events(vec![]);
        let token = get_admin_token_from_state(&state);
        let mut app = state.into_router();

        let event = Event::new(default_content());
        let content = event.content;

        let request = Request::builder()
            .method(http::Method::POST)
            .uri("/events")
            .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::from(serde_json::to_vec(&content).unwrap()))
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

    #[tokio::test]
    async fn retrieve_all_with_filter() {
        let event1 = EventContent {
            program_id: ProgramId::new("program1").unwrap(),
            event_name: Some("event1".to_string()),
            ..default_content()
        };
        let event2 = EventContent {
            program_id: ProgramId::new("program2").unwrap(),
            event_name: Some("event2".to_string()),
            ..default_content()
        };
        let event3 = EventContent {
            program_id: ProgramId::new("program3").unwrap(),
            event_name: Some("event3".to_string()),
            ..default_content()
        };

        let events = vec![Event::new(event1), Event::new(event2), Event::new(event3)];

        let state = state_with_events(events);
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
