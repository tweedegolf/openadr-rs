use std::collections::hash_map::Entry;

use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use reqwest::StatusCode;
use serde::Deserialize;
use tracing::{debug, info, trace, warn};
use validator::Validate;

use openadr::wire::program::{ProgramContent, ProgramId};
use openadr::wire::Program;
use openadr::wire::target::TargetLabel;

use crate::api::{AppResponse, ValidatedQuery};
use crate::error::AppError;
use crate::error::AppError::NotFound;
use crate::state::AppState;

pub async fn get_all(
    State(state): State<AppState>,
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
) -> AppResponse<Vec<Program>> {
    trace!(?query_params);
    let programs = state
        .programs
        .read()
        .await
        .values()
        .filter_map(|program| match query_params.matches(program) {
            Ok(true) => Some(Ok(program.clone())),
            Ok(false) => None,
            Err(err) => Some(Err(err)),
        })
        .skip(query_params.skip as usize)
        .take(query_params.limit as usize)
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(Json(programs))
}

#[tracing::instrument(skip(state))]
pub async fn get(State(state): State<AppState>, Path(id): Path<ProgramId>) -> AppResponse<Program> {
    Ok(Json(
        state
            .programs
            .read()
            .await
            .get(&id)
            .ok_or(NotFound)?
            .clone(),
    ))
}

// TODO
//   '409':
//   description: Conflict. Implementation dependent response if program with the same programName exists.
//   content:
//        application/json:
//        schema:
//        $ref: '#/components/schemas/problem'
pub async fn add(
    State(state): State<AppState>,
    Json(new_program): Json<ProgramContent>,
) -> Result<(StatusCode, Json<Program>), AppError> {
    let mut map = state.programs.write().await;

    if let Some((_, conflict)) = map
        .iter()
        .find(|(_, p)| p.content.program_name == new_program.program_name)
    {
        warn!(id=%conflict.id, program_name=%new_program.program_name, "Conflicting program_name");
        return Err(AppError::Conflict(format!(
            "Program with id {} has the same name",
            conflict.id
        )));
    }

    let program = Program::new(new_program);
    map.insert(program.id.clone(), program.clone());

    info!(%program.id,
        program.program_name=program.content.program_name,
        "program created"
    );

    Ok((StatusCode::CREATED, Json(program)))
}

pub async fn edit(
    State(state): State<AppState>,
    Path(id): Path<ProgramId>,
    Json(content): Json<ProgramContent>,
) -> AppResponse<Program> {
    let mut map = state.programs.write().await;
    if let Some((_, conflict)) = map
        .iter()
        .find(|(inner_id, p)| id != **inner_id && p.content.program_name == content.program_name)
    {
        warn!(updated=%id, conflicting=%conflict.id, program_name=%content.program_name, "Conflicting program_name");
        return Err(AppError::Conflict(format!(
            "Program with id {} has the same name",
            conflict.id
        )));
    }

    match map.entry(id) {
        Entry::Occupied(mut entry) => {
            let p = entry.get_mut();
            p.content = content;
            p.modification_date_time = Utc::now();

            info!(%p.id,
                    program.program_name=p.content.program_name,
                    "program updated"
            );

            Ok(Json(p.clone()))
        }
        Entry::Vacant(_) => Err(NotFound),
    }
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<ProgramId>,
) -> AppResponse<Program> {
    debug!(%id, "delete program");
    match state.programs.write().await.remove(&id) {
        None => Err(NotFound),
        Some(removed) => Ok(Json(removed)),
    }
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
