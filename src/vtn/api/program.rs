use std::cmp::min;
use std::collections::hash_map::Entry;

use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use reqwest::StatusCode;
use serde::Deserialize;
use tracing::{debug, trace};
use validator::Validate;

use openadr::wire::program::{ProgramContent, ProgramId};
use openadr::wire::target::TargetLabel;
use openadr::wire::Program;

use crate::api::{AppResponse, ValidatedQuery};
use crate::error::AppError;
use crate::error::AppError::NotFound;
use crate::state::AppState;

pub async fn get_all(
    State(state): State<AppState>,
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
) -> AppResponse<Vec<Program>> {
    trace!("Received query params: {:?}", query_params);

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
        .collect::<Result<Vec<_>, AppError>>()?;
    
    trace!("filtered programs: {:?}", programs);

    Ok(Json(
        programs
            .get(query_params.skip as usize..min((query_params.skip + query_params.limit) as usize, programs.len()))
            .unwrap_or(&[])
            .to_vec(),
    ))
}

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
    let program = Program::new(new_program);
    debug!("created program with name '{}' and id '{}'", program.content.program_name, program.id);
    state
        .programs
        .write()
        .await
        .insert(program.id.clone(), program.clone());
    Ok((StatusCode::CREATED, Json(program)))
}

// TODO
//   '409':
//   description: Conflict. Implementation dependent response if program with the same programName exists.
//   content:
//        application/json:
//        schema:
//        $ref: '#/components/schemas/problem'
pub async fn edit(
    State(state): State<AppState>,
    Path(id): Path<ProgramId>,
    Json(content): Json<ProgramContent>,
) -> AppResponse<Program> {
    let mut map = state.programs.write().await;
    match map.entry(id) {
        Entry::Occupied(mut entry) => {
            let p = entry.get_mut();
            p.content = content;
            p.modification_date_time = Utc::now();
            Ok(Json(p.clone()))
        }
        Entry::Vacant(_) => Err(NotFound),
    }
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<ProgramId>,
) -> AppResponse<Program> {
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
