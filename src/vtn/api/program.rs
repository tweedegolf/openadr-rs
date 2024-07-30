use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use reqwest::StatusCode;
use serde::Deserialize;
use tracing::{info, trace, warn};
use validator::Validate;

use openadr::wire::program::{ProgramContent, ProgramId};
use openadr::wire::target::TargetLabel;
use openadr::wire::Program;

use crate::api::{AppResponse, ValidatedQuery};
use crate::data_source::Crud;
use crate::error::AppError;
use crate::state::AppState;

impl Crud<Program> for AppState {
    type Id = ProgramId;
    type NewType = ProgramContent;
    type Error = AppError;
    type Filter = QueryParams;

    async fn create(&self, new: Self::NewType) -> Result<Program, Self::Error> {
        if let Some(conflict) = self
            .programs
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
        self.programs
            .write()
            .await
            .insert(program.id.clone(), program.clone());

        info!(%program.id,
            program.program_name=program.content.program_name,
            "program created"
        );

        Ok(program)
    }

    async fn retrieve(&self, id: &Self::Id) -> Result<Program, Self::Error> {
        self.programs
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or(AppError::NotFound)
    }

    async fn retrieve_all(&self, filter: &Self::Filter) -> Result<Vec<Program>, Self::Error> {
        self.programs
            .read()
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

    async fn update(&self, id: &Self::Id, content: Self::NewType) -> Result<Program, Self::Error> {
        if let Some((_, conflict)) =
            self.programs.read().await.iter().find(|(inner_id, p)| {
                id != *inner_id && p.content.program_name == content.program_name
            })
        {
            warn!(updated=%id, conflicting=%conflict.id, program_name=%content.program_name, "Conflicting program_name");
            return Err(AppError::Conflict(format!(
                "Program with id {} has the same name",
                conflict.id
            )));
        }

        match self.programs.write().await.get_mut(id) {
            Some(occupied) => {
                occupied.content = content;
                occupied.modification_date_time = Utc::now();
                Ok(occupied.clone())
            }
            None => Err(AppError::NotFound),
        }
    }

    async fn delete(&self, id: &Self::Id) -> Result<Program, Self::Error> {
        match self.programs.write().await.remove(id) {
            Some(program) => Ok(program),
            None => Err(AppError::NotFound),
        }
    }
}

pub async fn get_all(
    State(state): State<AppState>,
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
) -> AppResponse<Vec<Program>> {
    trace!(?query_params);

    let programs = <AppState as Crud<Program>>::retrieve_all(&state, &query_params).await?;

    Ok(Json(programs))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<ProgramId>) -> AppResponse<Program> {
    let program = <AppState as Crud<Program>>::retrieve(&state, &id).await?;
    Ok(Json(program))
}

pub async fn add(
    State(state): State<AppState>,
    Json(new_program): Json<ProgramContent>,
) -> Result<(StatusCode, Json<Program>), AppError> {
    let program = <AppState as Crud<Program>>::create(&state, new_program).await?;

    Ok((StatusCode::CREATED, Json(program)))
}

pub async fn edit(
    State(state): State<AppState>,
    Path(id): Path<ProgramId>,
    Json(content): Json<ProgramContent>,
) -> AppResponse<Program> {
    let program = <AppState as Crud<Program>>::update(&state, &id, content).await?;

    info!(%program.id, program.program_name=program.content.program_name, "program updated");

    Ok(Json(program))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<ProgramId>,
) -> AppResponse<Program> {
    let program = <AppState as Crud<Program>>::delete(&state, &id).await?;
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
