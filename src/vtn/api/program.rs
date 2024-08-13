use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::{async_trait, Json};
use chrono::Utc;
use reqwest::StatusCode;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{info, trace, warn};
use validator::Validate;

use openadr::wire::program::{ProgramContent, ProgramId};
use openadr::wire::target::TargetLabel;
use openadr::wire::Program;

use crate::api::{AppResponse, ValidatedQuery};
use crate::data_source::{Crud, ProgramCrud};
use crate::error::AppError;
use crate::jwt::{BLUser, User};

impl ProgramCrud for RwLock<HashMap<ProgramId, Program>> {}

#[async_trait]
impl Crud for RwLock<HashMap<ProgramId, Program>> {
    type Type = Program;
    type Id = ProgramId;
    type NewType = ProgramContent;
    type Error = AppError;
    type Filter = QueryParams;

    async fn create(&self, new: Self::NewType) -> Result<Self::Type, Self::Error> {
        if let Some(conflict) = self
            .read()
            .await
            .values()
            .find(|p| p.content.program_name == new.program_name)
        {
            warn!(id=%conflict.id, program_name=%new.program_name, "Conflicting program_name");
            return Err(AppError::Conflict(format!(
                "Program with id {} has the same name",
                conflict.id
            )));
        }

        let program = Program::new(new);
        self.write()
            .await
            .insert(program.id.clone(), program.clone());

        info!(%program.id,
            program.program_name=program.content.program_name,
            "program created"
        );

        Ok(program)
    }

    async fn retrieve(&self, id: &Self::Id) -> Result<Self::Type, Self::Error> {
        self.read().await.get(id).cloned().ok_or(AppError::NotFound)
    }

    async fn retrieve_all(&self, filter: &Self::Filter) -> Result<Vec<Self::Type>, Self::Error> {
        self.read()
            .await
            .values()
            .filter_map(|program| match filter.matches(program) {
                Ok(true) => Some(Ok(program.clone())),
                Ok(false) => None,
                Err(err) => Some(Err(err)),
            })
            .skip(filter.skip as usize)
            .take(filter.limit as usize)
            .collect::<Result<Vec<_>, AppError>>()
    }

    async fn update(
        &self,
        id: &Self::Id,
        content: Self::NewType,
    ) -> Result<Self::Type, Self::Error> {
        if let Some((_, conflict)) =
            self.read().await.iter().find(|(inner_id, p)| {
                id != *inner_id && p.content.program_name == content.program_name
            })
        {
            warn!(updated=%id, conflicting=%conflict.id, program_name=%content.program_name, "Conflicting program_name");
            return Err(AppError::Conflict(format!(
                "Program with id {} has the same name",
                conflict.id
            )));
        }

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
            Some(program) => Ok(program),
            None => Err(AppError::NotFound),
        }
    }
}

pub async fn get_all(
    State(program_source): State<Arc<dyn ProgramCrud>>,
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
    User(_user): User,
) -> AppResponse<Vec<Program>> {
    trace!(?query_params);

    let programs = program_source.retrieve_all(&query_params).await?;

    Ok(Json(programs))
}

pub async fn get(
    State(program_source): State<Arc<dyn ProgramCrud>>,
    Path(id): Path<ProgramId>,
    User(_user): User,
) -> AppResponse<Program> {
    let program = program_source.retrieve(&id).await?;
    Ok(Json(program))
}

pub async fn add(
    State(program_source): State<Arc<dyn ProgramCrud>>,
    BLUser(_user): BLUser,
    Json(new_program): Json<ProgramContent>,
) -> Result<(StatusCode, Json<Program>), AppError> {
    let program = program_source.create(new_program).await?;

    Ok((StatusCode::CREATED, Json(program)))
}

pub async fn edit(
    State(program_source): State<Arc<dyn ProgramCrud>>,
    Path(id): Path<ProgramId>,
    BLUser(_user): BLUser,
    Json(content): Json<ProgramContent>,
) -> AppResponse<Program> {
    let program = program_source.update(&id, content).await?;

    info!(%program.id, program.program_name=program.content.program_name, "program updated");

    Ok(Json(program))
}

pub async fn delete(
    State(program_source): State<Arc<dyn ProgramCrud>>,
    Path(id): Path<ProgramId>,
    BLUser(_user): BLUser,
) -> AppResponse<Program> {
    let program = program_source.delete(&id).await?;
    info!(%id, "deleted program");
    Ok(Json(program))
}

#[derive(Deserialize, Validate, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QueryParams {
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
    pub fn matches(&self, program: &Program) -> Result<bool, AppError> {
        if let Some(target_type) = self.target_type.clone() {
            return match target_type {
                TargetLabel::ProgramName => Ok(self
                    .target_values
                    .clone()
                    .ok_or(AppError::BadRequest(
                        "If targetType is specified, targetValues must be specified as well",
                    ))?
                    .into_iter()
                    .any(|name| name == program.content.program_name)),
                _ => Err(AppError::NotImplemented(
                    "Program can only be filtered by name",
                )),
            };
        }
        Ok(true)
    }
}

#[cfg(test)]
mod test {
    use crate::{data_source::InMemoryStorage, jwt::JwtManager, state::AppState};

    use super::*;
    use axum::{
        body::Body,
        http::{self, Request, Response, StatusCode},
        Router,
    };
    use http_body_util::BodyExt; // for `collect`
    use tower::{Service, ServiceExt}; // for `call`, `oneshot`, and `ready`

    fn default_content() -> ProgramContent {
        ProgramContent {
            object_type: None,
            program_name: "program_name".to_string(),
            program_long_name: Some("program_long_name".to_string()),
            retailer_name: Some("retailer_name".to_string()),
            retailer_long_name: Some("retailer_long_name".to_string()),
            program_type: None,
            country: None,
            principal_subdivision: None,
            time_zone_offset: None,
            interval_period: None,
            program_descriptions: None,
            binding_events: None,
            local_price: None,
            payload_descriptors: None,
            targets: None,
        }
    }

    fn program_request(method: http::Method, program: Program) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(format!("/programs/{}", program.id))
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::from(serde_json::to_vec(&program).unwrap()))
            .unwrap()
    }

    async fn state_with_programs(programs: Vec<Program>) -> AppState {
        let store = InMemoryStorage::default();

        for program in programs {
            store
                .programs
                .write()
                .await
                .insert(program.id.clone(), program);
        }

        AppState::new(store, JwtManager::from_base64_secret("test").unwrap())
    }

    #[tokio::test]
    async fn get() {
        let program = Program::new(default_content());
        let program_id = program.id.clone();

        let state = state_with_programs(vec![program.clone()]).await;
        let app = crate::app_with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::GET)
                    .uri(format!("/programs/{program_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let db_program: Program = serde_json::from_slice(&body).unwrap();

        assert_eq!(program, db_program);
    }

    #[tokio::test]
    async fn delete() {
        let program1 = ProgramContent {
            program_name: "program1".to_string(),
            ..default_content()
        };
        let program2 = ProgramContent {
            program_name: "program2".to_string(),
            ..default_content()
        };
        let program3 = ProgramContent {
            program_name: "program3".to_string(),
            ..default_content()
        };

        let programs = vec![
            Program::new(program1),
            Program::new(program2.clone()),
            Program::new(program3),
        ];
        let program_id = programs[1].id.clone();

        let state = state_with_programs(programs).await;
        let mut app = crate::app_with_state(state);

        let request = Request::builder()
            .method(http::Method::DELETE)
            .uri(format!("/programs/{program_id}"))
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
        let db_program: Program = serde_json::from_slice(&body).unwrap();

        assert_eq!(program2, db_program.content);

        let response = retrieve_all_with_filter_help(&mut app, "").await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let programs: Vec<Program> = serde_json::from_slice(&body).unwrap();
        assert_eq!(programs.len(), 2);
    }

    #[tokio::test]
    async fn update() {
        let program = Program::new(default_content());

        let state = state_with_programs(vec![program.clone()]).await;
        let app = crate::app_with_state(state);

        let response = app
            .oneshot(program_request(http::Method::PUT, program.clone()))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let db_program: Program = serde_json::from_slice(&body).unwrap();

        assert_eq!(program.content, db_program.content);
        assert!(program.modification_date_time < db_program.modification_date_time);
    }

    #[tokio::test]
    async fn update_same_name() {
        let program = Program::new(default_content());

        let state = state_with_programs(vec![program.clone()]).await;
        let app = crate::app_with_state(state);

        // different id, same (default) name
        let program = Program::new(default_content());

        let response = app
            .oneshot(program_request(http::Method::PUT, program.clone()))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn create_same_name() {
        let state = state_with_programs(vec![]).await;
        let mut app = crate::app_with_state(state);

        let program = Program::new(default_content());
        let content = program.content;

        let request = Request::builder()
            .method(http::Method::POST)
            .uri("/programs")
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
            .uri("/programs")
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::from(serde_json::to_vec(&content).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    async fn retrieve_all_with_filter_help(app: &mut Router, query_params: &str) -> Response<Body> {
        let request = Request::builder()
            .method(http::Method::GET)
            .uri(format!("/programs?{query_params}"))
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
        let program1 = ProgramContent {
            program_name: "program1".to_string(),
            ..default_content()
        };
        let program2 = ProgramContent {
            program_name: "program2".to_string(),
            ..default_content()
        };
        let program3 = ProgramContent {
            program_name: "program3".to_string(),
            ..default_content()
        };

        let programs = vec![
            Program::new(program1),
            Program::new(program2),
            Program::new(program3),
        ];

        let state = state_with_programs(programs).await;
        let mut app = crate::app_with_state(state);

        // no query params
        let response = retrieve_all_with_filter_help(&mut app, "").await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let programs: Vec<Program> = serde_json::from_slice(&body).unwrap();
        assert_eq!(programs.len(), 3);

        // skip
        let response = retrieve_all_with_filter_help(&mut app, "skip=1").await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let programs: Vec<Program> = serde_json::from_slice(&body).unwrap();
        assert_eq!(programs.len(), 2);

        // limit
        let response = retrieve_all_with_filter_help(&mut app, "limit=2").await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let programs: Vec<Program> = serde_json::from_slice(&body).unwrap();
        assert_eq!(programs.len(), 2);

        // program name
        let response = retrieve_all_with_filter_help(&mut app, "targetType=NONSENSE").await;
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);

        let response = retrieve_all_with_filter_help(
            &mut app,
            "targetType=PROGRAM_NAME&targetValues=program1&targetValues=program2",
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let programs: Vec<Program> = serde_json::from_slice(&body).unwrap();
        assert_eq!(programs.len(), 2);
    }
}
